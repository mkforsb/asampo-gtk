// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use libasampo::{
    samples::Sample,
    samplesets::{export::ExportJobMessage, DrumkitLabel, SampleSet},
    sequences::{
        drumkit_render_thread::Message as DrtMessage, DrumkitSequence, DrumkitSequenceEvent,
    },
    sources::{file_system_source::FilesystemSource, Source, SourceOps},
};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    view::{dialogs::ExportDialogView, sequences::DrumMachineView},
};

mod core;
pub(in crate::model) mod delegate;
mod drum_machine;
mod signals;
mod viewflags;
mod viewvalues;

use delegate::{delegate, delegate_priv};

use core::{CoreModel, SourceLoadMsg};
use signals::SignalModel;
use viewflags::ViewFlags;
use viewvalues::ViewValues;

pub use core::ExportState;
pub use drum_machine::{DrumMachineModel, Mirroring, PlaybackState as DrumMachinePlaybackState};
pub use signals::Signal;
pub use viewvalues::ExportKind;

pub type AnyhowResult<T> = Result<T, anyhow::Error>;

#[derive(Clone, Debug)]
pub struct AppModel {
    core: CoreModel,
    core_orig: CoreModel,
    viewflags: ViewFlags,
    viewvalues: ViewValues,
    signals: SignalModel,
    audiothread_tx: mpsc::Sender<audiothread::Message>,
    drum_machine: DrumMachineModel,
    drum_machine_orig: DrumMachineModel,
}

pub type AppModelPtr = Rc<Cell<Option<AppModel>>>;

impl AppModel {
    pub fn new(
        config: AppConfig,
        savefile: Option<String>,
        audiothread_tx: mpsc::Sender<audiothread::Message>,
    ) -> Self {
        let viewvalues = ViewValues::new(&config);
        let drum_machine = DrumMachineModel::new_with_render_thread(audiothread_tx.clone());

        AppModel {
            core: CoreModel::new(config.clone(), savefile.clone()),
            core_orig: CoreModel::new(config, savefile),
            viewflags: ViewFlags::default(),
            viewvalues,
            signals: SignalModel::new(),
            audiothread_tx,
            drum_machine: drum_machine.clone(),
            drum_machine_orig: drum_machine,
        }
    }

    pub fn tap<F: FnOnce(&AppModel)>(self, f: F) -> AppModel {
        f(&self);
        self
    }

    pub fn modified(&self) -> bool {
        self.core != self.core_orig || self.drum_machine != self.drum_machine_orig
    }

    pub fn reset_modified_state(self) -> AppModel {
        AppModel {
            core_orig: self.core.clone(),
            drum_machine_orig: self.drum_machine.clone(),
            ..self
        }
    }

    pub fn spawn_audiothread(
        config: &AppConfig,
    ) -> AnyhowResult<mpsc::Sender<audiothread::Message>> {
        let (audiothread_tx, audiothread_rx) = mpsc::channel::<audiothread::Message>();

        let _ = audiothread::spawn(
            audiothread_rx,
            Some(
                audiothread::Opts::default()
                    .with_name("asampo")
                    .with_spec(audiothread::AudioSpec::new(config.output_samplerate_hz, 2)?)
                    .with_conversion_quality(config.sample_rate_conversion_quality)
                    .with_buffer_size((config.buffer_size_frames as usize).try_into()?),
            ),
        );

        Ok(audiothread_tx)
    }

    pub fn audiothread_send(&self, message: audiothread::Message) -> AnyhowResult<()> {
        self.audiothread_tx
            .send(message)
            .map_err(|e| anyhow!("Audiothread send error: {e}"))
    }

    pub fn drum_machine_model(&self) -> &DrumMachineModel {
        &self.drum_machine
    }

    pub fn reconfigure(self, config: AppConfig) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        let new_audiothread_tx = AppModel::spawn_audiothread(&config)?;

        // TODO: retain sequence, samples, tempo etc.
        let new_drum_machine_model =
            DrumMachineModel::new_with_render_thread(new_audiothread_tx.clone());

        let old_audiothread_tx = std::mem::replace(&mut result.audiothread_tx, new_audiothread_tx);
        let old_drum_machine_model =
            std::mem::replace(&mut result.drum_machine, new_drum_machine_model);

        let (old_drum_machine_tx, old_drum_machine_event_rx) = old_drum_machine_model.take_comms();

        // spawn a thread to allow graceful shutdown of old threads
        std::thread::spawn(move || {
            if let Some(tx) = &old_drum_machine_tx {
                match tx.send(DrtMessage::Shutdown) {
                    Ok(_) => (),
                    Err(e) => {
                        log::log!(
                            log::Level::Error,
                            "Error shutting down drumkit sequence render thread: {e}"
                        );
                    }
                }
            }

            // give drum machine render time some time to disconnect and shut down gracefully
            std::thread::sleep(Duration::from_millis(250));
            drop(old_drum_machine_event_rx);

            match old_audiothread_tx.send(audiothread::Message::Shutdown) {
                Ok(_) => {
                    // give audiothread some time to shut down gracefully
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(e) => {
                    log::log!(log::Level::Error, "Error shutting down audiothread: {e}")
                }
            }
        });

        Ok(result.set_config(config))
    }

    fn add_fs_source_fields_valid(model: &AppModel) -> bool {
        !(model.add_fs_source_name().is_empty()
            || model.add_fs_source_path().is_empty()
            || model.add_fs_source_extensions().is_empty())
    }

    pub fn validate_add_fs_source_fields(self) -> AppModel {
        let valid = Self::add_fs_source_fields_valid(&self);
        self.set_add_fs_source_fields_valid(valid)
    }

    pub fn commit_file_system_source(self) -> AnyhowResult<AppModel> {
        if Self::add_fs_source_fields_valid(&self) {
            let name = self.add_fs_source_name().clone();
            let path = self.add_fs_source_path().clone();
            let exts = self
                .add_fs_source_extensions()
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            self.add_file_system_source(name, path, exts)
        } else {
            Err(anyhow!(
                "Failed to commit file system source: invalid field(s)"
            ))
        }
    }

    // TODO: more validation, e.g is the path readable
    fn add_file_system_source(
        self,
        name: String,
        path: String,
        exts: Vec<String>,
    ) -> AnyhowResult<AppModel> {
        let new_source = Source::FilesystemSource(FilesystemSource::new_named(name, path, exts));
        let uuid = *new_source.uuid();

        Ok(self
            .init_source_sample_count(uuid)?
            .add_source(new_source)?
            .enable_source(uuid)?
            .clear_add_fs_source_fields()
            .set_add_fs_source_fields_valid(false))
    }

    pub fn load_sources(self, sources: Vec<Source>) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        for source in sources {
            let uuid = *source.uuid();
            let enabled = source.is_enabled();

            result = result.init_source_sample_count(uuid)?.add_source(source)?;

            if enabled {
                result = result.enable_source(uuid)?;
            }
        }

        Ok(result)
    }

    pub fn remove_source(self, uuid: Uuid) -> AnyhowResult<AppModel> {
        self.remove_source_core(uuid)?
            .remove_source_sample_count(uuid)
    }

    pub fn clear_sources(self) -> AppModel {
        self.clear_sources_core().clear_sources_sample_counts()
    }

    pub fn handle_source_loader(
        self,
        source_uuid: Uuid,
        messages: Vec<SourceLoadMsg>,
    ) -> AnyhowResult<AppModel> {
        let len_before = self.samples().len();
        self.handle_source_loader_core(messages);

        let added = self.samples().len() - len_before;
        self.source_sample_count_add(source_uuid, added)
    }

    pub fn populate_samples_listmodel(&self) {
        self.viewvalues.populate_samples_listmodel(&self.samples());
    }

    pub fn get_or_create_set(
        model: AppModel,
        set_name: impl Into<String>,
    ) -> AnyhowResult<(AppModel, Uuid)> {
        let (result, uuid) = CoreModel::get_or_create_set(model.core, set_name)?;

        Ok((
            AppModel {
                core: result,
                ..model
            },
            uuid,
        ))
    }

    pub fn load_sets(self, sets: Vec<SampleSet>) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        for set in sets {
            result = result.add_set(set)?
        }

        Ok(result)
    }

    pub fn clear_sets(self) -> AppModel {
        self.clear_sets_core()
            .set_add_to_prev_set_enabled(false)
            .set_set_export_enabled(false)
            .reset_export_progress()
    }

    pub fn load_sequences(self, seqs: Vec<DrumkitSequence>) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        for seq in seqs {
            result = result.add_sequence(seq)?
        }

        Ok(result)
    }

    //////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////

    delegate!(core,
        set_config(config: AppConfig): Model,
        config() -> &AppConfig,
        set_config_save_timeout(deadline: Instant): Model,
        clear_config_save_timeout(): Model,
        reached_config_save_timeout() -> bool,
        set_savefile_path(maybe_path: Option<impl Into<String>>): Model,
        savefile_path() -> Option<&String>,
        source(uuid: Uuid) -> AnyhowResult<&Source>,
        sources_map() -> &HashMap<Uuid, Source>,
        sources_list() -> Vec<&Source>,
        add_source(source: Source): Result,
        enable_source(uuid: Uuid): Result,
        disable_source(uuid: Uuid): Result,
        source_loaders() -> &HashMap<Uuid, Rc<mpsc::Receiver<SourceLoadMsg>>>,
        remove_source_loader(uuid: Uuid): Result,
        has_sources_loading() -> bool,
        samples() -> std::cell::Ref<Vec<Sample>>,
        set_selected_sample(s: Option<Sample>): Model,
        selected_sample() -> Option<&Sample>,
        sets_map() -> &HashMap<Uuid, SampleSet>,
        sets_list() -> Vec<&SampleSet>,
        set(uuid: Uuid) -> AnyhowResult<&SampleSet>,
        add_set(set: SampleSet): Result,
        remove_set(uuid: Uuid): Result,
        insert_set(set: SampleSet, position: usize): Result,
        add_to_set(sample: Sample, set_uuid: Uuid): Result,
        remove_from_set(sample: &Sample, set_uuid: Uuid): Result,
        set_sample_label(set_uuid: Uuid, sample: Sample, label: Option<DrumkitLabel>): Result,
        set_set_most_recently_added_to(maybe_uuid: Option<Uuid>): Result,
        set_most_recently_added_to() -> Option<Uuid>,
        set_selected_set(maybe_uuid: Option<Uuid>): Result,
        set_selected_set_member(maybe_sample: Option<Sample>): Model,
        selected_set_member() -> Option<&Sample>,
        selected_set() -> Option<Uuid>,
        set_export_state(maybe_state: Option<ExportState>): Model,
        export_state() -> Option<ExportState>,
        set_export_job_rx(rx: Option<mpsc::Receiver<ExportJobMessage>>): Model,
        export_job_rx() -> Option<Rc<mpsc::Receiver<ExportJobMessage>>>,
        sequence(uuid: Uuid) -> AnyhowResult<&DrumkitSequence>,
        sequences_list() -> Vec<&DrumkitSequence>,
        sequences_map() -> &HashMap<Uuid, DrumkitSequence>,
        add_sequence(seq: DrumkitSequence): Result,
        insert_sequence(seq: DrumkitSequence, position: usize): Result,
        set_selected_sequence(maybe_uuid: Option<Uuid>): Result,
        selected_sequence() -> Option<Uuid>,
        remove_sequence(uuid: Uuid): Result,
        clear_sequences(): Model);

    delegate_priv!(core,
        remove_source(uuid: Uuid) as remove_source_core: Result,
        clear_sources() as clear_sources_core: Model,
        handle_source_loader(messages: Vec<SourceLoadMsg>) as handle_source_loader_core -> (),
        clear_sets() as clear_sets_core: Model);

    delegate!(viewflags,
        set_main_view_sensitive(sensitive: bool): Model,
        is_main_view_sensitive() -> bool,
        set_add_fs_source_fields_valid(valid: bool): Model,
        are_add_fs_source_fields_valid() -> bool,
        set_export_fields_valid(valid: bool): Model,
        are_export_fields_valid() -> bool,
        set_set_load_in_drum_machine_enabled(state: bool): Model,
        is_set_load_in_drum_machine_enabled() -> bool,
        set_set_export_enabled(state: bool): Model,
        is_set_export_enabled() -> bool,
        set_add_to_prev_set_enabled(state: bool): Model,
        is_add_to_prev_set_enabled() -> bool);

    delegate!(viewvalues,
        set_latency_approx_label_by_config(config: &AppConfig): Model,
        latency_approx_label() -> &String,
        sources_sample_count() -> &HashMap<Uuid, usize>,
        init_source_sample_count(source_uuid: Uuid): Result,
        source_sample_count_add(source_uuid: Uuid, add: usize): Result,
        reset_source_sample_count(source_uuid: Uuid): Result,
        remove_source_sample_count(source_uuid: Uuid): Result,
        clear_sources_sample_counts(): Model,
        set_add_fs_source_name(text: impl Into<String>): Model,
        set_add_fs_source_name_if_empty(text: impl Into<String>): Model,
        add_fs_source_name() -> &String,
        set_add_fs_source_path(text: impl Into<String>): Model,
        add_fs_source_path() -> &String,
        set_add_fs_source_extensions(text: impl Into<String>): Model,
        add_fs_source_extensions() -> &String,
        clear_add_fs_source_fields(): Model,
        get_listed_sample(index: u32) -> Result<Sample, anyhow::Error>,
        set_samples_list_filter(text: impl Into<String>): Model,
        set_set_pending_deletion(maybe_uuid: Option<Uuid>): Model,
        set_pending_deletion() -> Option<Uuid>,
        set_export_dialog_view(view: Option<ExportDialogView>): Model,
        export_dialog_view() -> Option<&ExportDialogView>,
        set_export_target_dir(text: impl Into<String>): Model,
        export_target_dir() -> &String,
        set_export_kind(kind: ExportKind): Model,
        export_kind() -> &ExportKind,
        init_export_progress(total_items: usize): Model,
        export_progress() -> Option<(usize, usize)>,
        set_export_items_completed(completed: usize): Result,
        reset_export_progress(): Model,
        samples_listmodel() -> &gtk::gio::ListStore,
        set_drum_machine_view(view: Option<DrumMachineView>): Model,
        drum_machine_view() -> Option<&DrumMachineView>,
        set_sequence_pending_deletion(maybe_uuid: Option<Uuid>): Model,
        sequence_pending_deletion() -> Option<Uuid>);

    delegate!(signals,
        signal(signal: Signal): Model,
        clear_signal(signal: Signal): Result,
        is_signalling(signal: Signal) -> bool);

    delegate!(drum_machine,
        is_render_thread_active() as is_drum_machine_render_thread_active -> bool,
        render_thread_send(message: DrtMessage) as drum_machine_send -> Result<(), anyhow::Error>,
        set_activated_pad(n: usize) as set_activated_drum_machine_pad: Result,
        activated_pad() as activated_drum_machine_pad -> usize,
        set_activated_part(n: usize) as set_activated_drum_machine_part: Result,
        truncate_parts_to(n: usize) as truncate_drum_machine_parts_to: Result,
        activated_part() as activated_drum_machine_part -> usize,
        sequence() as drum_machine_sequence -> &DrumkitSequence,
        loaded_sequence() as drum_machine_loaded_sequence -> Option<&DrumkitSequence>,
        clear_loaded_sequence() as clear_drum_machine_loaded_sequence: Model,
        load_sampleset(set: SampleSet, sources: Vec<Source>) as load_drum_machine_sampleset: Result,
        loaded_sampleset() as drum_machine_loaded_sampleset -> Option<&SampleSet>,
        clear_loaded_sampleset() as clear_drum_machine_loaded_sampleset: Model,
        clear_sampleset() as clear_drum_machine_sampleset: Result,
        set_sampleset(set: SampleSet, src: Vec<Source>, mi: Mirroring)
            as set_drum_machine_sampleset: Result,
        commit_sampleset() as commit_drum_machine_sampleset: Result,
        swap_to_saved_sampleset(saved_set: SampleSet) as swap_drum_machine_sampleset: Model,
        sampleset() as drum_machine_sampleset -> &SampleSet,
        load_sequence(sequence: DrumkitSequence) as load_drum_machine_sequence: Result,
        commit_sequence() as commit_drum_machine_sequence: Result,
        swap_to_saved_sequence(saved_seq: DrumkitSequence) as swap_drum_machine_sequence: Model,
        set_sequence(sequence: DrumkitSequence, mirroring: Mirroring)
            as set_drum_machine_sequence: Result,
        clear_sequence() as clear_drum_machine_sequence: Result,
        set_tempo(bpm: u16, mirroring: Mirroring) as set_drum_machine_tempo: Result,
        set_swing(swing: f64, mirroring: Mirroring) as set_drum_machine_swing: Result,
        set_latest_event(event: Option<DrumkitSequenceEvent>)
            as set_latest_drum_machine_event: Model,
        poll_event() as drum_machine_poll_event -> Option<DrumkitSequenceEvent>,
        play() as drum_machine_play: Result,
        pause() as drum_machine_pause: Result,
        stop() as drum_machine_stop: Result,
        rewind() as drum_machine_rewind -> AnyhowResult<()>,
        playback_state() as drum_machine_playback_state -> DrumMachinePlaybackState,
        assign_sample(source: &Source, sample: Sample, label: DrumkitLabel)
            as assign_drum_pad: Result);
}
