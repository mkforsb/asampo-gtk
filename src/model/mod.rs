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
    sequences::{drumkit_render_thread, DrumkitSequence, DrumkitSequenceEvent},
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
mod viewflags;
mod viewvalues;

use delegate::{delegate, delegate_priv};

use core::{CoreModel, SourceLoaderMessage};
use viewflags::ViewFlags;
use viewvalues::ViewValues;

pub use core::ExportState;
pub use drum_machine::{DrumMachineModel, Mirroring, PlaybackState as DrumMachinePlaybackState};
pub use viewvalues::ExportKind;

pub type AnyhowResult<T> = Result<T, anyhow::Error>;

#[derive(Clone, Debug)]
pub struct AppModel {
    core: CoreModel,
    viewflags: ViewFlags,
    viewvalues: ViewValues,
    audiothread_tx: mpsc::Sender<audiothread::Message>,
    drum_machine: DrumMachineModel,
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
            core: CoreModel::new(config, savefile),
            viewflags: ViewFlags::default(),
            viewvalues,
            audiothread_tx,
            drum_machine,
        }
    }

    pub fn tap<F: FnOnce(&AppModel)>(self, f: F) -> AppModel {
        f(&self);
        self
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

    delegate!(core, set_config(config: AppConfig) -> Model);
    delegate!(core, config() -> &AppConfig);
    delegate!(core, set_config_save_timeout(deadline: Instant) -> Model);
    delegate!(core, clear_config_save_timeout() -> Model);
    delegate!(core, reached_config_save_timeout() -> bool);

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
                match tx.send(drumkit_render_thread::Message::Shutdown) {
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

    delegate!(core, set_savefile_path(maybe_path: Option<impl Into<String>>) -> Model);
    delegate!(core, savefile_path() -> Option<&String>);
    delegate!(core, source(uuid: Uuid) -> AnyhowResult<&Source>);
    delegate!(core, sources_map() -> &HashMap<Uuid, Source>);
    delegate!(core, sources_list() -> Vec<&Source>);
    delegate!(core, add_source(source: Source) -> Result);

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

    delegate!(core, enable_source(uuid: Uuid) -> Result);
    delegate!(core, disable_source(uuid: Uuid) -> Result);
    delegate_priv!(core, remove_source(uuid: Uuid) as remove_source_core -> Result);

    pub fn remove_source(self, uuid: Uuid) -> AnyhowResult<AppModel> {
        self.remove_source_core(uuid)?
            .remove_source_sample_count(uuid)
    }

    delegate_priv!(core, clear_sources() as clear_sources_core -> Model);

    pub fn clear_sources(self) -> AppModel {
        self.clear_sources_core().clear_sources_sample_counts()
    }

    delegate!(core, source_loaders() ->
        &HashMap<Uuid, Rc<mpsc::Receiver<Result<Sample, libasampo::errors::Error>>>>);

    delegate_priv!(core, handle_source_loader(messages: Vec<SourceLoaderMessage>)
        as handle_source_loader_core -> ());

    pub fn handle_source_loader(
        self,
        source_uuid: Uuid,
        messages: Vec<SourceLoaderMessage>,
    ) -> AnyhowResult<AppModel> {
        let len_before = self.samples().len();
        self.handle_source_loader_core(messages);

        let added = self.samples().len() - len_before;
        self.source_sample_count_add(source_uuid, added)
    }

    delegate!(core, remove_source_loader(uuid: Uuid) -> Result);
    delegate!(core, has_sources_loading() -> bool);
    delegate!(core, samples() -> std::cell::Ref<Vec<Sample>>);

    pub fn populate_samples_listmodel(&self) {
        self.viewvalues.populate_samples_listmodel(&self.samples());
    }

    delegate!(core, set_selected_sample(s: Option<Sample>) -> Model);
    delegate!(core, selected_sample() -> Option<&Sample>);
    delegate!(core, sets_list() -> Vec<&SampleSet>);
    delegate!(core, sets_map() -> &HashMap<Uuid, SampleSet>);
    delegate!(core, set(uuid: Uuid) -> AnyhowResult<&SampleSet>);
    delegate!(core, add_set(set: SampleSet) -> Result);

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

    delegate_priv!(core, clear_sets() as clear_sets_core -> Model);

    pub fn clear_sets(self) -> AppModel {
        self.clear_sets_core()
            .set_add_to_prev_set_enabled(false)
            .set_set_export_enabled(false)
            .reset_export_progress()
    }

    delegate!(core, remove_set(uuid: Uuid) -> Result);
    delegate!(core, insert_set(set: SampleSet, position: usize) -> Result);

    delegate!(core, add_to_set(sample: Sample, set_uuid: Uuid) -> Result);

    delegate!(core,
        set_set_sample_label(set_uuid: Uuid, sample: Sample, label: Option<DrumkitLabel>)
        -> Result);

    delegate!(core, set_most_recently_added_to() -> Option<Uuid>);
    delegate!(core, set_selected_set(maybe_uuid: Option<Uuid>) -> Result);
    delegate!(core, set_selected_set_member(maybe_sample: Option<Sample>) -> Model);
    delegate!(core, selected_set_member() -> Option<&Sample>);
    delegate!(core, selected_set() -> Option<Uuid>);
    delegate!(core, set_export_state(maybe_state: Option<ExportState>) -> Model);
    delegate!(core, export_state() -> Option<ExportState>);
    delegate!(core, set_export_job_rx(rx: Option<mpsc::Receiver<ExportJobMessage>>) -> Model);
    delegate!(core, export_job_rx() -> Option<Rc<mpsc::Receiver<ExportJobMessage>>>);
    delegate!(core, sequence(uuid: Uuid) -> AnyhowResult<&DrumkitSequence>);
    delegate!(core, sequences_list() -> Vec<&DrumkitSequence>);
    delegate!(core, sequences_map() -> &HashMap<Uuid, DrumkitSequence>);
    delegate!(core, add_sequence(seq: DrumkitSequence) -> Result);
    delegate!(core, insert_sequence(seq: DrumkitSequence, position: usize) -> Result);
    delegate!(core, set_selected_sequence(maybe_uuid: Option<Uuid>) -> Result);
    delegate!(core, selected_sequence() -> Option<Uuid>);
    delegate!(core, remove_sequence(uuid: Uuid) -> Result);
    delegate!(core, clear_sequences() -> Model);

    pub fn load_sequences(self, seqs: Vec<DrumkitSequence>) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        for seq in seqs {
            result = result.add_sequence(seq)?
        }

        Ok(result)
    }

    delegate!(viewflags, set_main_view_sensitive(sensitive: bool) -> Model);
    delegate!(viewflags, is_main_view_sensitive() -> bool);
    delegate!(viewflags, set_add_fs_source_fields_valid(valid: bool) -> Model);
    delegate!(viewflags, are_add_fs_source_fields_valid() -> bool);
    delegate!(viewflags, set_export_fields_valid(valid: bool) -> Model);
    delegate!(viewflags, are_export_fields_valid() -> bool);
    delegate!(viewflags, set_set_load_in_drum_machine_enabled(state: bool) -> Model);
    delegate!(viewflags, is_set_load_in_drum_machine_enabled() -> bool);
    delegate!(viewflags, set_set_export_enabled(state: bool) -> Model);
    delegate!(viewflags, is_set_export_enabled() -> bool);
    delegate!(viewflags, set_add_to_prev_set_enabled(state: bool) -> Model);
    delegate!(viewflags, is_add_to_prev_set_enabled() -> bool);
    delegate!(viewflags, signal_add_fs_source_begin_browse() -> Model);
    delegate!(viewflags, signal_add_sample_to_set_show_dialog() -> Model);
    delegate!(viewflags, signal_add_set_show_dialog() -> Model);
    delegate!(viewflags, signal_export_begin_browse() -> Model);
    delegate!(viewflags, signal_export_show_dialog() -> Model);
    delegate!(viewflags, signal_create_sequence_show_dialog() -> Model);
    delegate!(viewflags, signal_sequence_save_as_show_dialog() -> Model);
    delegate!(viewflags, signal_sampleset_save_as_show_dialog() -> Model);
    delegate!(viewflags, signal_sequence_load_show_confirm_save_dialog() -> Model);
    delegate!(viewflags, signal_sequence_load_show_confirm_abandon_dialog() -> Model);
    delegate!(viewflags, signal_sequence_clear_show_confirm_dialog() -> Model);
    delegate!(viewflags, is_signalling_add_fs_source_begin_browse() -> bool);
    delegate!(viewflags, is_signalling_add_sample_to_set_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_add_set_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_export_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_export_begin_browse() -> bool);
    delegate!(viewflags, is_signalling_create_sequence_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_sequence_save_as_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_sampleset_save_as_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_sequence_load_show_confirm_save_dialog() -> bool);
    delegate!(viewflags, is_signalling_sequence_load_show_confirm_abandon_dialog() -> bool);
    delegate!(viewflags, is_signalling_sequence_clear_show_confirm_dialog() -> bool);
    delegate!(viewflags, clear_signal_add_fs_source_begin_browse() -> Model);
    delegate!(viewflags, clear_signal_add_sample_to_set_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_add_set_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_export_begin_browse() -> Model);
    delegate!(viewflags, clear_signal_export_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_create_sequence_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_sequence_save_as_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_sampleset_save_as_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_sequence_load_show_confirm_save_dialog() -> Model);
    delegate!(viewflags, clear_signal_sequence_load_show_confirm_abandon_dialog() -> Model);
    delegate!(viewflags, clear_signal_sequence_clear_show_confirm_dialog() -> Model);

    delegate!(viewvalues, set_latency_approx_label_by_config(config: &AppConfig) -> Model);
    delegate!(viewvalues, latency_approx_label() -> &String);
    delegate!(viewvalues, sources_sample_count() -> &HashMap<Uuid, usize>);
    delegate!(viewvalues, init_source_sample_count(source_uuid: Uuid) -> Result);
    delegate!(viewvalues, source_sample_count_add(source_uuid: Uuid, add: usize) -> Result);
    delegate!(viewvalues, reset_source_sample_count(source_uuid: Uuid) -> Result);
    delegate!(viewvalues, remove_source_sample_count(source_uuid: Uuid) -> Result);
    delegate!(viewvalues, clear_sources_sample_counts() -> Model);
    delegate!(viewvalues, set_add_fs_source_name(text: impl Into<String>) -> Model);
    delegate!(viewvalues, set_add_fs_source_name_if_empty(text: impl Into<String>) -> Model);
    delegate!(viewvalues, add_fs_source_name() -> &String);
    delegate!(viewvalues, set_add_fs_source_path(text: impl Into<String>) -> Model);
    delegate!(viewvalues, add_fs_source_path() -> &String);
    delegate!(viewvalues, set_add_fs_source_extensions(text: impl Into<String>) -> Model);
    delegate!(viewvalues, add_fs_source_extensions() -> &String);
    delegate!(viewvalues, clear_add_fs_source_fields() -> Model);
    delegate!(viewvalues, get_listed_sample(index: u32) -> Result<Sample, anyhow::Error>);
    delegate!(viewvalues, set_samples_list_filter(text: impl Into<String>) -> Model);
    delegate!(viewvalues, set_export_dialog_view(view: Option<ExportDialogView>) -> Model);
    delegate!(viewvalues, export_dialog_view() -> Option<&ExportDialogView>);
    delegate!(viewvalues, set_export_target_dir(text: impl Into<String>) -> Model);
    delegate!(viewvalues, export_target_dir() -> &String);
    delegate!(viewvalues, set_export_kind(kind: ExportKind) -> Model);
    delegate!(viewvalues, export_kind() -> &ExportKind);
    delegate!(viewvalues, init_export_progress(total_items: usize) -> Model);
    delegate!(viewvalues, export_progress() -> Option<(usize, usize)>);
    delegate!(viewvalues, set_export_items_completed(completed: usize) -> Result);
    delegate!(viewvalues, reset_export_progress() -> Model);
    delegate!(viewvalues, samples_listmodel() -> &gtk::gio::ListStore);
    delegate!(viewvalues, set_drum_machine_view(view: Option<DrumMachineView>)
        -> Model);
    delegate!(viewvalues, drum_machine_view() -> Option<&DrumMachineView>);

    delegate!(drum_machine, is_render_thread_active()
        as is_drum_machine_render_thread_active -> bool);

    delegate!(drum_machine, render_thread_send(message: drumkit_render_thread::Message)
        as drum_machine_render_thread_send -> Result<(), anyhow::Error>);

    delegate!(drum_machine, set_activated_pad(n: usize)
        as set_activated_drum_machine_pad -> Result);

    delegate!(drum_machine, activated_pad() as activated_drum_machine_pad -> usize);

    delegate!(drum_machine, set_activated_part(n: usize)
        as set_activated_drum_machine_part -> Result);

    delegate!(drum_machine, activated_part() as activated_drum_machine_part -> usize);

    delegate!(drum_machine, sequence() as drum_machine_sequence -> &DrumkitSequence);

    delegate!(drum_machine, loaded_sequence()
        as drum_machine_loaded_sequence -> Option<&DrumkitSequence>);

    delegate!(drum_machine, clear_loaded_sequence() as clear_drum_machine_loaded_sequence -> Model);

    delegate!(drum_machine, load_sampleset(set: SampleSet, sources: Vec<Source>)
        as load_drum_machine_sampleset -> Result);

    delegate!(drum_machine, loaded_sampleset()
        as drum_machine_loaded_sampleset -> Option<&SampleSet>);

    delegate!(drum_machine, clear_loaded_sampleset()
        as clear_drum_machine_loaded_sampleset -> Model);

    delegate!(
        drum_machine,
        set_sampleset(set: SampleSet, sources: Vec<Source>, mirroring: Mirroring)
        as set_drum_machine_sampleset -> Result
    );

    delegate!(drum_machine, commit_sampleset() as commit_drum_machine_sampleset -> Result);

    delegate!(drum_machine, swap_to_saved_sampleset(saved_set: SampleSet)
        as swap_drum_machine_sampleset -> Model);

    delegate!(drum_machine, sampleset() as drum_machine_sampleset -> &SampleSet);

    delegate!(drum_machine, load_sequence(sequence: DrumkitSequence)
        as load_drum_machine_sequence -> Result);

    delegate!(drum_machine, commit_sequence() as commit_drum_machine_sequence -> Result);

    delegate!(drum_machine, swap_to_saved_sequence(saved_seq: DrumkitSequence)
        as swap_drum_machine_sequence -> Model);

    delegate!(
        drum_machine,
        set_sequence(sequence: DrumkitSequence, mirroring: Mirroring)
        as set_drum_machine_sequence -> Result);

    delegate!(drum_machine, clear_sequence() as clear_drum_machine_sequence -> Result);

    delegate!(drum_machine, set_tempo(bpm: u16, mirroring: Mirroring)
        as set_drum_machine_tempo -> Result);

    delegate!(drum_machine, set_swing(swing: f64, mirroring: Mirroring)
        as set_drum_machine_swing -> Result);

    delegate!(drum_machine, set_latest_event(event: Option<DrumkitSequenceEvent>)
        as set_latest_drum_machine_event -> Model);

    delegate!(drum_machine, poll_event()
        as drum_machine_poll_event -> Option<DrumkitSequenceEvent>);

    delegate!(drum_machine, play() as drum_machine_play -> Result);
    delegate!(drum_machine, pause() as drum_machine_pause -> Result);
    delegate!(drum_machine, stop() as drum_machine_stop -> Result);
    delegate!(drum_machine, rewind() as drum_machine_rewind -> AnyhowResult<()>);
    delegate!(
        drum_machine,
        playback_state() as drum_machine_playback_state -> DrumMachinePlaybackState);

    delegate!(drum_machine, assign_sample(source: &Source, sample: Sample, label: DrumkitLabel)
        as assign_drum_pad -> Result);
}
