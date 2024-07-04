// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use gtk::{glib::clone, prelude::ListModelExt};
use libasampo::{
    samples::{Sample, SampleOps},
    samplesets::{export::ExportJobMessage, SampleSet, SampleSetLabelling, SampleSetOps},
    sequences::{drumkit_render_thread, DrumkitSequence, DrumkitSequenceEvent},
    sources::{file_system_source::FilesystemSource, Source, SourceOps},
};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    ext::{ClonedHashMapExt, ClonedVecExt},
    model::{delegate::delegate, DrumMachineModel, ExportKind, ViewFlags, ViewValues},
    view::{dialogs::ExportDialogView, samples::SampleListEntry},
};

type AnyhowResult<T> = Result<T, anyhow::Error>;
type SourceLoaderMessage = Result<Sample, libasampo::errors::Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportState {
    Exporting,
    Finished,
}

#[derive(Clone, Debug)]
pub struct AppModel {
    pub config: AppConfig,
    pub config_save_timeout: Option<std::time::Instant>,
    pub savefile: Option<String>,
    pub viewflags: ViewFlags,
    pub viewvalues: ViewValues,
    pub audiothread_tx: mpsc::Sender<audiothread::Message>,
    pub sources: HashMap<Uuid, Source>,
    pub sources_order: Vec<Uuid>,
    pub sources_loading: HashMap<Uuid, Rc<mpsc::Receiver<SourceLoaderMessage>>>,
    pub samples: Rc<RefCell<Vec<Sample>>>,
    pub samplelist_selected_sample: Option<Sample>,
    pub sets: HashMap<Uuid, SampleSet>,
    pub sets_order: Vec<Uuid>,
    pub sets_selected_set: Option<Uuid>,
    pub sets_most_recently_used_uuid: Option<Uuid>,
    pub sets_export_state: Option<ExportState>,
    pub export_job_rx: Option<Rc<mpsc::Receiver<ExportJobMessage>>>,
    pub drum_machine: DrumMachineModel,
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
            config,
            config_save_timeout: None,
            savefile,
            viewflags: ViewFlags::default(),
            viewvalues,
            audiothread_tx,
            sources: HashMap::new(),
            sources_order: Vec::new(),
            sources_loading: HashMap::new(),
            samples: Rc::new(RefCell::new(Vec::new())),
            samplelist_selected_sample: None,
            sets: HashMap::new(),
            sets_order: Vec::new(),
            sets_selected_set: None,
            sets_most_recently_used_uuid: None,
            sets_export_state: None,
            export_job_rx: None,
            drum_machine,
        }
    }

    pub fn disable_source(self, uuid: Uuid) -> AnyhowResult<AppModel> {
        self.samples
            .borrow_mut()
            .retain(|s| s.source_uuid() != Some(&uuid));

        Ok(AppModel {
            sources: self
                .sources
                .cloned_update_with(|mut s: HashMap<Uuid, Source>| {
                    s.get_mut(&uuid)
                        .ok_or_else(|| anyhow!("Failed to disable source: uuid not found!"))?
                        .disable();
                    Ok(s)
                })?,
            ..self
        })
    }

    pub fn remove_source(self, uuid: Uuid) -> AnyhowResult<AppModel> {
        let model = self
            .disable_source(uuid)?
            .remove_source_sample_count(uuid)?;

        Ok(AppModel {
            sources_order: model.sources_order.clone_and_remove(&uuid)?,
            sources: model.sources.clone_and_remove(&uuid)?,
            ..model
        })
    }

    pub fn populate_samples_listmodel(&self) {
        let filter = &self.viewvalues.samples_list_filter;
        self.viewvalues.samples_listview_model.remove_all();

        if filter.is_empty() {
            let samples = self
                .samples
                .borrow()
                .iter()
                .map(|s| SampleListEntry::new(s.clone()))
                .collect::<Vec<_>>();

            self.viewvalues
                .samples_listview_model
                .extend_from_slice(samples.as_slice());
        } else {
            let fragments = filter
                .split(' ')
                .map(|s| s.to_string().to_lowercase())
                .collect::<Vec<_>>();

            let mut samples = self.samples.borrow().clone();

            samples.retain(|x| {
                fragments
                    .iter()
                    .all(|frag| x.uri().as_str().to_lowercase().contains(frag))
            });

            self.viewvalues.samples_listview_model.extend_from_slice(
                samples
                    .iter()
                    .map(|s| SampleListEntry::new(s.clone()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
        }

        log::log!(
            log::Level::Debug,
            "Showing {} samples",
            self.viewvalues.samples_listview_model.n_items()
        );
    }

    pub fn add_sampleset(self, set: SampleSet) -> AppModel {
        AppModel {
            sets_order: self.sets_order.clone_and_push(*set.uuid()),
            sets: self.sets.clone_and_insert(*set.uuid(), set),
            ..self
        }
    }

    #[cfg(test)]
    pub fn remove_sampleset(self, uuid: &Uuid) -> AnyhowResult<AppModel> {
        Ok(AppModel {
            sets_order: self.sets_order.clone_and_remove(uuid)?,
            sets: self.sets.clone_and_remove(uuid)?,
            ..self
        })
    }

    pub fn set_config(self, config: AppConfig) -> AppModel {
        AppModel { config, ..self }
    }

    pub fn set_config_save_timeout(self, deadline: Instant) -> AppModel {
        AppModel {
            config_save_timeout: Some(deadline),
            ..self
        }
    }

    pub fn clear_config_save_timeout(self) -> AppModel {
        AppModel {
            config_save_timeout: None,
            ..self
        }
    }

    pub fn add_source(self, source: Source) -> AnyhowResult<AppModel> {
        debug_assert!(self.sources.len() == self.sources_order.len());
        debug_assert!(self
            .sources
            .iter()
            .all(|(_uuid, source)| self.sources_order.iter().any(|uuid| source.uuid() == uuid)));

        if self.sources.contains_key(source.uuid()) {
            Err(anyhow!("Failed to add source: UUID in use"))
        } else {
            Ok(AppModel {
                sources_order: self.sources_order.clone_and_push(*source.uuid()),
                sources: self.sources.clone_and_insert(*source.uuid(), source),
                ..self
            })
        }
    }

    pub fn add_source_loader(
        self,
        source_uuid: Uuid,
        loader_rx: mpsc::Receiver<SourceLoaderMessage>,
    ) -> AnyhowResult<AppModel> {
        if self.sources_loading.contains_key(&source_uuid) {
            Err(anyhow!("Failed to add source loader: UUID in use"))
        } else {
            Ok(AppModel {
                sources_loading: self
                    .sources_loading
                    .clone_and_insert(source_uuid, Rc::new(loader_rx)),
                ..self
            })
        }
    }

    pub fn remove_source_loader(self, source_uuid: Uuid) -> AnyhowResult<AppModel> {
        if !self.sources_loading.contains_key(&source_uuid) {
            Err(anyhow!("Failed to remove source loader: UUID not present"))
        } else {
            Ok(AppModel {
                sources_loading: self.sources_loading.clone_and_remove(&source_uuid)?,
                ..self
            })
        }
    }

    pub fn enable_source(self, uuid: &Uuid) -> AnyhowResult<AppModel> {
        Ok(AppModel {
            sources: self
                .sources
                .cloned_update_with(|mut s: HashMap<Uuid, Source>| {
                    s.get_mut(uuid)
                        .ok_or_else(|| anyhow!("Failed to enable source: UUID not present"))?
                        .enable();
                    Ok(s)
                })?,
            ..self
        })
    }

    pub fn has_sources_loading(&self) -> bool {
        !self.sources_loading.is_empty()
    }

    pub fn reached_config_save_timeout(&self) -> bool {
        self.config_save_timeout
            .is_some_and(|t| t <= Instant::now())
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn audiothread_send(&self, message: audiothread::Message) -> AnyhowResult<()> {
        self.audiothread_tx
            .send(message)
            .map_err(|e| anyhow!("Audiothread send error: {e}"))
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

    pub fn handle_source_loader(
        self,
        source_uuid: Uuid,
        messages: Vec<SourceLoaderMessage>,
    ) -> AnyhowResult<AppModel> {
        let mut samples = self.samples.borrow_mut();
        let len_before = samples.len();

        for message in messages {
            match message {
                Ok(sample) => {
                    samples.push(sample);
                }

                Err(e) => log::log!(log::Level::Error, "Error loading source: {e}"),
            }
        }

        let added = samples.len() - len_before;
        drop(samples);

        // TODO: this goes in AppModel while the above goes in CoreModel
        self.source_sample_count_add(source_uuid, added)
    }

    pub fn source(&self, uuid: Uuid) -> AnyhowResult<&Source> {
        self.sources
            .get(&uuid)
            .ok_or(anyhow!("Failed to get source: UUID not present"))
    }

    pub fn set_selected_sample(self, maybe_sample: Option<Sample>) -> AppModel {
        AppModel {
            samplelist_selected_sample: maybe_sample,
            ..self
        }
    }

    pub fn get_set_most_recently_added_to(&self) -> Option<Uuid> {
        self.sets_most_recently_used_uuid
    }

    fn sources_add_fs_fields_valid(model: &AppModel) -> bool {
        !(model.viewvalues.sources_add_fs_name_entry.is_empty()
            || model.viewvalues.sources_add_fs_path_entry.is_empty()
            || model.viewvalues.sources_add_fs_extensions_entry.is_empty())
    }

    pub fn validate_sources_add_fs_fields(self) -> AppModel {
        let valid = Self::sources_add_fs_fields_valid(&self);
        self.set_are_sources_add_fs_fields_valid(valid)
    }

    pub fn commit_file_system_source(self) -> AnyhowResult<AppModel> {
        if Self::sources_add_fs_fields_valid(&self) {
            let name = self.viewvalues.sources_add_fs_name_entry.clone();
            let path = self.viewvalues.sources_add_fs_path_entry.clone();
            let exts = self
                .viewvalues
                .sources_add_fs_extensions_entry
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
    pub fn add_file_system_source(
        self,
        name: String,
        path: String,
        exts: Vec<String>,
    ) -> AnyhowResult<AppModel> {
        let new_source = Source::FilesystemSource(FilesystemSource::new_named(name, path, exts));
        let uuid = *new_source.uuid();

        let (loader_tx, loader_rx) = mpsc::channel::<Result<Sample, libasampo::errors::Error>>();

        std::thread::spawn(clone!(@strong new_source => move || {
            new_source.list_async(loader_tx);
        }));

        self.init_source_sample_count(uuid)?
            .add_source(new_source.clone())?
            .enable_source(&uuid)?
            .clear_sources_add_fs_fields()
            .set_are_sources_add_fs_fields_valid(false)
            .add_source_loader(uuid, loader_rx)
    }

    pub fn conditionally<P, F>(self, cond: P, op: F) -> AppModel
    where
        P: FnOnce() -> bool,
        F: FnOnce(AppModel) -> AppModel,
    {
        if cond() {
            op(self)
        } else {
            self
        }
    }

    // pub fn conditionally_fallible<P, F, T>(self, cond: P, op: F) -> AnyhowResult<AppModel>
    // where
    //     P: FnOnce() -> bool,
    //     F: FnOnce(AppModel) -> AnyhowResult<AppModel>,
    // {
    //     if cond() {
    //         op(self)
    //     } else {
    //         Ok(self)
    //     }
    // }

    pub fn tap<F: FnOnce(&AppModel)>(self, f: F) -> AppModel {
        f(&self);
        self
    }

    pub fn clear_sources(self) -> AppModel {
        AppModel {
            sources: HashMap::new(),
            sources_order: Vec::new(),
            sources_loading: HashMap::new(),
            samples: Rc::new(RefCell::new(Vec::new())),
            ..self
        }
        .clear_sources_sample_counts()
    }

    pub fn clear_sets(self) -> AppModel {
        AppModel {
            sets: HashMap::new(),
            sets_order: Vec::new(),
            sets_selected_set: None,
            sets_most_recently_used_uuid: None,
            sets_export_state: None,
            ..self
        }
        .disable_set_export()
        .reset_export_progress()
    }

    pub fn load_sources(self, sources: Vec<Source>) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        for source in sources {
            let uuid = *source.uuid();
            let loader_rx = Self::load_source(source.clone());

            result = result
                .add_source(source)?
                .init_source_sample_count(uuid)?
                .add_source_loader(uuid, loader_rx)?;
        }

        Ok(result)
    }

    fn load_source(source: Source) -> mpsc::Receiver<SourceLoaderMessage> {
        let (tx, rx) = mpsc::channel::<SourceLoaderMessage>();

        std::thread::spawn(move || {
            source.list_async(tx);
        });

        rx
    }

    fn add_set(self, set: SampleSet) -> AnyhowResult<AppModel> {
        if self.sets.contains_key(set.uuid()) {
            Err(anyhow!("Failed to add set: UUID in use"))
        } else {
            let uuid = *set.uuid();

            Ok(AppModel {
                sets: self.sets.clone_and_insert(uuid, set),
                sets_order: self.sets_order.clone_and_push(uuid),
                ..self
            })
        }
    }

    pub fn load_sets(self, sets: Vec<SampleSet>) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        for set in sets {
            result = result.add_set(set)?
        }

        Ok(result)
    }

    pub fn set_savefile_path(self, maybe_path: Option<String>) -> AppModel {
        AppModel {
            savefile: maybe_path,
            ..self
        }
    }

    pub fn get_set(&self, uuid: Uuid) -> AnyhowResult<&SampleSet> {
        self.sets
            .get(&uuid)
            .ok_or(anyhow!("Failed to fetch sample set: UUID not present"))
    }

    pub fn set_selected_set(self, maybe_uuid: Option<Uuid>) -> AnyhowResult<AppModel> {
        if let Some(false) = maybe_uuid.map(|uuid| self.sets.contains_key(&uuid)) {
            Err(anyhow!("Failed to set selected set: UUID not present"))
        } else {
            Ok(AppModel {
                sets_selected_set: maybe_uuid,
                ..self
            })
        }
    }

    pub fn get_selected_set(&self) -> Option<Uuid> {
        self.sets_selected_set
    }

    pub fn set_labelling(
        self,
        set_uuid: Uuid,
        labelling: Option<SampleSetLabelling>,
    ) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        result
            .sets
            .get_mut(&set_uuid)
            .ok_or(anyhow!("Failed to set labelling: UUID not present"))?
            .set_labelling(labelling);

        Ok(result)
    }

    pub fn sources(&self) -> &HashMap<Uuid, Source> {
        &self.sources
    }

    pub fn set_export_state(self, maybe_state: Option<ExportState>) -> AppModel {
        AppModel {
            sets_export_state: maybe_state,
            ..self
        }
    }

    pub fn set_export_job_rx(self, maybe_rx: Option<mpsc::Receiver<ExportJobMessage>>) -> AppModel {
        AppModel {
            export_job_rx: maybe_rx.map(Rc::new),
            ..self
        }
    }

    // TODO: replace this with something more abstract
    pub fn set_drum_machine(self, drum_machine: DrumMachineModel) -> AppModel {
        AppModel {
            drum_machine,
            ..self
        }
    }

    delegate!(viewflags, set_are_sources_add_fs_fields_valid(valid: bool) -> Model);
    delegate!(viewflags, signal_sources_add_fs_begin_browse() -> Model);
    delegate!(viewflags, clear_signal_sources_add_fs_begin_browse() -> Model);
    delegate!(viewflags, signal_add_sample_to_set_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_add_sample_to_set_show_dialog() -> Model);
    delegate!(viewflags, enable_set_export() -> Model);
    delegate!(viewflags, disable_set_export() -> Model);
    delegate!(viewflags, signal_add_set_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_add_set_show_dialog() -> Model);
    delegate!(viewflags, signal_export_begin_browse() -> Model);
    delegate!(viewflags, clear_signal_export_begin_browse() -> Model);
    delegate!(viewflags, signal_export_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_export_show_dialog() -> Model);
    delegate!(viewflags, set_main_view_sensitive(sensitive: bool) -> Model);
    delegate!(viewflags, set_are_export_fields_valid(valid: bool) -> Model);
    delegate!(viewflags, is_main_view_sensitive() -> bool);

    // delegate!(viewvalues, set_latency_approx_label(text: String) -> Model);
    delegate!(viewvalues, set_latency_approx_label_by_config(config: &AppConfig) -> Model);
    delegate!(viewvalues, init_source_sample_count(source_uuid: Uuid) -> Result);
    delegate!(viewvalues, source_sample_count_add(source_uuid: Uuid, add: usize) -> Result);
    delegate!(viewvalues, reset_source_sample_count(source_uuid: Uuid) -> Result);
    delegate!(viewvalues, remove_source_sample_count(source_uuid: Uuid) -> Result);
    delegate!(viewvalues, clear_sources_add_fs_fields() -> Model);
    delegate!(viewvalues, set_sources_add_fs_name_entry(text: impl Into<String>) -> Model);
    delegate!(viewvalues, set_sources_add_fs_name_entry_if_empty(text: impl Into<String>) -> Model);
    delegate!(viewvalues, set_sources_add_fs_path_entry(text: impl Into<String>) -> Model);
    delegate!(viewvalues, set_sources_add_fs_extensions_entry(text: impl Into<String>) -> Model);
    delegate!(viewvalues, get_listed_sample(index: u32) -> Result<Sample, anyhow::Error>);
    delegate!(viewvalues, set_samples_list_filter_text(text: impl Into<String>) -> Model);
    delegate!(viewvalues, clear_sources_sample_counts() -> Model);
    delegate!(viewvalues, set_export_dialog_view(view: Option<ExportDialogView>) -> Model);
    delegate!(viewvalues, set_export_target_dir_entry_text(text: impl Into<String>) -> Model);
    delegate!(viewvalues, set_export_kind(kind: ExportKind) -> Model);
    delegate!(viewvalues, init_export_progress(total_items: usize) -> Model);
    delegate!(viewvalues, set_export_items_completed(completed: usize) -> Result);
    delegate!(viewvalues, reset_export_progress() -> Model);
    delegate!(viewvalues, export_target_dir() -> &String);
    delegate!(viewvalues, export_kind() -> &ExportKind);

    delegate!(drum_machine, is_render_thread_active()
        as is_drum_machine_render_thread_active -> bool);

    delegate!(drum_machine, render_thread_send(message: drumkit_render_thread::Message)
        as drum_machine_render_thread_send -> Result<(), anyhow::Error>);

    delegate!(drum_machine, set_activated_pad(n: usize)
        as set_activated_drum_machine_pad -> Result);

    delegate!(drum_machine, activated_pad() as activated_drum_machine_pad -> usize);
    delegate!(drum_machine, sequence() as drum_machine_sequence -> &DrumkitSequence);

    delegate!(drum_machine, set_sequence(sequence: DrumkitSequence)
        as set_drum_machine_sequence -> Model);

    delegate!(drum_machine, set_latest_event(event: Option<DrumkitSequenceEvent>)
        as set_latest_drum_machine_event -> Model);

    // delegate!(drum_machine, latest_event()
    //     as latest_drum_machine_event -> Option<&DrumkitSequenceEvent>);
}

#[cfg(test)]
mod tests {
    use libasampo::samplesets::BaseSampleSet;

    use super::*;

    #[test]
    fn test_add_remove_sampleset() {
        let (dummy_tx, _) = mpsc::channel::<audiothread::Message>();
        let model = AppModel::new(AppConfig::default(), None, dummy_tx);
        let set = BaseSampleSet::new("Favorites".to_string());

        let model = model.add_sampleset(SampleSet::BaseSampleSet(set.clone()));

        assert!(model.sets.contains_key(set.uuid()));
        assert_eq!(model.sets.get(set.uuid()).unwrap().name(), "Favorites");

        let model = model.remove_sampleset(set.uuid()).unwrap();

        assert!(!model.sets.contains_key(set.uuid()));
    }
}
