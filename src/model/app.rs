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
use libasampo::{
    samples::{Sample, SampleOps},
    samplesets::{
        export::ExportJobMessage, BaseSampleSet, SampleSet, SampleSetLabelling, SampleSetOps,
    },
    sequences::{drumkit_render_thread, DrumkitSequence, DrumkitSequenceEvent},
    sources::{file_system_source::FilesystemSource, Source, SourceOps},
};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    ext::{ClonedHashMapExt, ClonedVecExt},
    model::{delegate::delegate, DrumMachineModel, ExportKind, ViewFlags, ViewValues},
    view::{dialogs::ExportDialogView, sequences::DrumMachineView},
};

type AnyhowResult<T> = Result<T, anyhow::Error>;
type SourceLoaderMessage = Result<Sample, libasampo::errors::Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportState {
    Exporting,
    Finished,
}

#[derive(Clone, Debug)]
pub struct CoreModel {
    config: AppConfig,
    config_save_timeout: Option<std::time::Instant>,
    savefile: Option<String>,
    sources: HashMap<Uuid, Source>,
    sources_order: Vec<Uuid>,
    sources_loading: HashMap<Uuid, Rc<mpsc::Receiver<SourceLoaderMessage>>>,
    samples: Rc<RefCell<Vec<Sample>>>,
    samplelist_selected_sample: Option<Sample>,
    sets: HashMap<Uuid, SampleSet>,
    sets_order: Vec<Uuid>,
    sets_selected_set: Option<Uuid>,
    sets_most_recently_used_uuid: Option<Uuid>,
    sets_export_state: Option<ExportState>,
    export_job_rx: Option<Rc<mpsc::Receiver<ExportJobMessage>>>,
}

impl CoreModel {
    pub fn new(config: AppConfig, savefile_path: Option<impl Into<String>>) -> CoreModel {
        CoreModel {
            config,
            config_save_timeout: None,
            savefile: savefile_path.map(|s| s.into()),
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
        }
    }

    pub fn set_config(self, config: AppConfig) -> CoreModel {
        CoreModel { config, ..self }
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn set_config_save_timeout(self, deadline: Instant) -> CoreModel {
        CoreModel {
            config_save_timeout: Some(deadline),
            ..self
        }
    }

    pub fn clear_config_save_timeout(self) -> CoreModel {
        CoreModel {
            config_save_timeout: None,
            ..self
        }
    }

    pub fn reached_config_save_timeout(&self) -> bool {
        self.config_save_timeout
            .is_some_and(|t| t <= Instant::now())
    }

    pub fn set_savefile_path(self, maybe_path: Option<impl Into<String>>) -> CoreModel {
        CoreModel {
            savefile: maybe_path.map(|s| s.into()),
            ..self
        }
    }

    pub fn savefile_path(&self) -> Option<&String> {
        self.savefile.as_ref()
    }

    pub fn sources_map(&self) -> &HashMap<Uuid, Source> {
        &self.sources
    }

    pub fn sources_list(&self) -> Vec<&Source> {
        self.sources_order
            .iter()
            .map(|uuid| self.source(*uuid).unwrap())
            .collect()
    }

    pub fn source(&self, uuid: Uuid) -> AnyhowResult<&Source> {
        self.sources
            .get(&uuid)
            .ok_or(anyhow!("Failed to get source: UUID not present"))
    }

    pub fn add_source(self, source: Source) -> AnyhowResult<CoreModel> {
        debug_assert!(self.sources.len() == self.sources_order.len());
        debug_assert!(self
            .sources
            .iter()
            .all(|(_uuid, source)| self.sources_order.iter().any(|uuid| source.uuid() == uuid)));

        if self.sources.contains_key(source.uuid()) {
            Err(anyhow!("Failed to add source: UUID in use"))
        } else {
            Ok(CoreModel {
                sources_order: self.sources_order.clone_and_push(*source.uuid()),
                sources: self.sources.clone_and_insert(*source.uuid(), source),
                ..self
            })
        }
    }

    pub fn enable_source(self, uuid: Uuid) -> AnyhowResult<CoreModel> {
        let loader_rx = Self::spawn_source_loader(self.source(uuid)?.clone());

        CoreModel {
            sources: self
                .sources
                .cloned_update_with(|mut s: HashMap<Uuid, Source>| {
                    s.get_mut(&uuid)
                        .ok_or_else(|| anyhow!("Failed to enable source: UUID not present"))?
                        .enable();
                    Ok(s)
                })?,
            ..self
        }
        .add_source_loader(uuid, loader_rx)
    }

    pub fn disable_source(self, uuid: Uuid) -> AnyhowResult<CoreModel> {
        self.samples
            .borrow_mut()
            .retain(|s| s.source_uuid() != Some(&uuid));

        Ok(CoreModel {
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

    pub fn remove_source(self, uuid: Uuid) -> AnyhowResult<CoreModel> {
        let model = self.disable_source(uuid)?;

        Ok(CoreModel {
            sources_order: model.sources_order.clone_and_remove(&uuid)?,
            sources: model.sources.clone_and_remove(&uuid)?,
            ..model
        })
    }

    pub fn clear_sources(self) -> CoreModel {
        CoreModel {
            sources: HashMap::new(),
            sources_order: Vec::new(),
            sources_loading: HashMap::new(),
            samples: Rc::new(RefCell::new(Vec::new())),
            ..self
        }
    }

    fn spawn_source_loader(source: Source) -> mpsc::Receiver<SourceLoaderMessage> {
        let (tx, rx) = mpsc::channel::<SourceLoaderMessage>();

        std::thread::spawn(move || {
            source.list_async(tx);
        });

        rx
    }

    pub fn source_loaders(
        &self,
    ) -> &HashMap<Uuid, Rc<mpsc::Receiver<Result<Sample, libasampo::errors::Error>>>> {
        &self.sources_loading
    }

    pub fn add_source_loader(
        self,
        source_uuid: Uuid,
        loader_rx: mpsc::Receiver<SourceLoaderMessage>,
    ) -> AnyhowResult<CoreModel> {
        if self.sources_loading.contains_key(&source_uuid) {
            Err(anyhow!("Failed to add source loader: UUID in use"))
        } else {
            Ok(CoreModel {
                sources_loading: self
                    .sources_loading
                    .clone_and_insert(source_uuid, Rc::new(loader_rx)),
                ..self
            })
        }
    }

    pub fn handle_source_loader(&self, messages: Vec<SourceLoaderMessage>) {
        let mut samples = self.samples.borrow_mut();

        for message in messages {
            match message {
                Ok(sample) => {
                    samples.push(sample);
                }

                Err(e) => log::log!(log::Level::Error, "Error loading source: {e}"),
            }
        }
    }

    pub fn remove_source_loader(self, source_uuid: Uuid) -> AnyhowResult<CoreModel> {
        if !self.sources_loading.contains_key(&source_uuid) {
            Err(anyhow!("Failed to remove source loader: UUID not present"))
        } else {
            Ok(CoreModel {
                sources_loading: self.sources_loading.clone_and_remove(&source_uuid)?,
                ..self
            })
        }
    }

    pub fn has_sources_loading(&self) -> bool {
        !self.sources_loading.is_empty()
    }

    pub fn samples(&self) -> std::cell::Ref<Vec<Sample>> {
        self.samples.borrow()
    }

    pub fn set_selected_sample(self, maybe_sample: Option<Sample>) -> CoreModel {
        CoreModel {
            samplelist_selected_sample: maybe_sample,
            ..self
        }
    }

    pub fn selected_sample(&self) -> Option<&Sample> {
        self.samplelist_selected_sample.as_ref()
    }

    pub fn sets_list(&self) -> Vec<&SampleSet> {
        self.sets_order
            .iter()
            .map(|uuid| self.sets.get(uuid).unwrap())
            .collect()
    }

    pub fn sets_map(&self) -> &HashMap<Uuid, SampleSet> {
        &self.sets
    }

    pub fn get_set(&self, uuid: Uuid) -> AnyhowResult<&SampleSet> {
        self.sets
            .get(&uuid)
            .ok_or(anyhow!("Failed to fetch sample set: UUID not present"))
    }

    fn get_set_mut(&mut self, uuid: Uuid) -> AnyhowResult<&mut SampleSet> {
        self.sets
            .get_mut(&uuid)
            .ok_or(anyhow!("Failed to fetch sample set: UUID not present"))
    }

    pub fn add_set(self, set: SampleSet) -> AnyhowResult<CoreModel> {
        if self.sets.contains_key(set.uuid()) {
            Err(anyhow!("Failed to add set: UUID in use"))
        } else {
            let uuid = *set.uuid();

            Ok(CoreModel {
                sets: self.sets.clone_and_insert(uuid, set),
                sets_order: self.sets_order.clone_and_push(uuid),
                ..self
            })
        }
    }

    pub fn get_or_create_sampleset(
        model: CoreModel,
        name: impl Into<String>,
    ) -> Result<(CoreModel, Uuid), anyhow::Error> {
        let name = name.into();

        match model
            .sets
            .iter()
            .find(|(_, set)| set.name() == name)
            .map(|(uuid, _)| *uuid)
        {
            Some(uuid) => Ok((model, uuid)),
            None => {
                let new_set = SampleSet::BaseSampleSet(BaseSampleSet::new(name));
                let new_uuid = *new_set.uuid();

                Ok((model.add_set(new_set)?, new_uuid))
            }
        }
    }

    #[cfg(test)]
    pub fn remove_set(self, uuid: Uuid) -> AnyhowResult<CoreModel> {
        Ok(CoreModel {
            sets_order: self.sets_order.clone_and_remove(&uuid)?,
            sets: self.sets.clone_and_remove(&uuid)?,
            ..self
        })
    }

    pub fn clear_sets(self) -> CoreModel {
        CoreModel {
            sets: HashMap::new(),
            sets_order: Vec::new(),
            sets_selected_set: None,
            sets_most_recently_used_uuid: None,
            sets_export_state: None,
            ..self
        }
    }

    pub fn add_to_set(self, sample: Sample, set_uuid: Uuid) -> AnyhowResult<CoreModel> {
        let mut result = self.clone();

        result.get_set_mut(set_uuid)?.add(
            self.source(
                *sample
                    .source_uuid()
                    .ok_or(anyhow!("Sample missing source UUID"))?,
            )?,
            sample,
        )?;

        result.set_set_most_recently_added_to(Some(set_uuid))
    }

    fn set_set_most_recently_added_to(self, maybe_uuid: Option<Uuid>) -> AnyhowResult<CoreModel> {
        match maybe_uuid.and_then(|uuid| self.get_set(uuid).err()) {
            Some(err) => Err(err),
            None => Ok(CoreModel {
                sets_most_recently_used_uuid: maybe_uuid,
                ..self
            }),
        }
    }

    pub fn get_set_most_recently_added_to(&self) -> Option<Uuid> {
        self.sets_most_recently_used_uuid
    }

    pub fn set_selected_set(self, maybe_uuid: Option<Uuid>) -> AnyhowResult<CoreModel> {
        if let Some(false) = maybe_uuid.map(|uuid| self.sets.contains_key(&uuid)) {
            Err(anyhow!("Failed to set selected set: UUID not present"))
        } else {
            Ok(CoreModel {
                sets_selected_set: maybe_uuid,
                ..self
            })
        }
    }

    pub fn selected_set(&self) -> Option<Uuid> {
        self.sets_selected_set
    }

    pub fn set_labelling(
        self,
        set_uuid: Uuid,
        labelling: Option<SampleSetLabelling>,
    ) -> AnyhowResult<CoreModel> {
        let mut result = self.clone();

        result
            .sets
            .get_mut(&set_uuid)
            .ok_or(anyhow!("Failed to set labelling: UUID not present"))?
            .set_labelling(labelling);

        Ok(result)
    }

    pub fn set_export_state(self, maybe_state: Option<ExportState>) -> CoreModel {
        CoreModel {
            sets_export_state: maybe_state,
            ..self
        }
    }

    pub fn export_state(&self) -> Option<ExportState> {
        self.sets_export_state
    }

    pub fn set_export_job_rx(
        self,
        maybe_rx: Option<mpsc::Receiver<ExportJobMessage>>,
    ) -> CoreModel {
        CoreModel {
            export_job_rx: maybe_rx.map(Rc::new),
            ..self
        }
    }

    pub fn export_job_rx(&self) -> Option<Rc<mpsc::Receiver<ExportJobMessage>>> {
        self.export_job_rx.clone()
    }
}

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

    pub fn remove_source(self, uuid: Uuid) -> AnyhowResult<AppModel> {
        self.remove_source_core(uuid)?
            .remove_source_sample_count(uuid)
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
        let len_before = self.samples().len();
        self.handle_source_loader_core(messages);

        let added = self.samples().len() - len_before;
        self.source_sample_count_add(source_uuid, added)
    }

    fn sources_add_fs_fields_valid(model: &AppModel) -> bool {
        !(model.add_fs_source_name_entry_text().is_empty()
            || model.add_fs_source_path_entry_text().is_empty()
            || model.add_fs_source_extensions_entry_text().is_empty())
    }

    pub fn validate_sources_add_fs_fields(self) -> AppModel {
        let valid = Self::sources_add_fs_fields_valid(&self);
        self.set_are_sources_add_fs_fields_valid(valid)
    }

    pub fn commit_file_system_source(self) -> AnyhowResult<AppModel> {
        if Self::sources_add_fs_fields_valid(&self) {
            let name = self.add_fs_source_name_entry_text().clone();
            let path = self.add_fs_source_path_entry_text().clone();
            let exts = self
                .add_fs_source_extensions_entry_text()
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

        Ok(self
            .init_source_sample_count(uuid)?
            .add_source(new_source)?
            .enable_source(uuid)?
            .clear_sources_add_fs_fields()
            .set_are_sources_add_fs_fields_valid(false))
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
        self.clear_sources_core().clear_sources_sample_counts() // goes in appmodel, above in coremodel
    }

    pub fn clear_sets(self) -> AppModel {
        self.clear_sets_core()
            .disable_add_to_prev_set()
            .disable_set_export()
            .reset_export_progress()
    }

    pub fn load_sources(self, sources: Vec<Source>) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        for source in sources {
            let uuid = *source.uuid();

            result = result
                .init_source_sample_count(uuid)?
                .add_source(source)?
                .enable_source(uuid)?;
        }

        Ok(result)
    }

    pub fn load_sets(self, sets: Vec<SampleSet>) -> AnyhowResult<AppModel> {
        let mut result = self.clone();

        for set in sets {
            result = result.add_set(set)?
        }

        Ok(result)
    }

    // TODO: replace this with something more abstract
    pub fn set_drum_machine(self, drum_machine: DrumMachineModel) -> AppModel {
        AppModel {
            drum_machine,
            ..self
        }
    }

    pub fn populate_samples_listmodel(&self) {
        self.viewvalues.populate_samples_listmodel(&self.samples());
    }

    pub fn drum_machine_model(&self) -> &DrumMachineModel {
        &self.drum_machine
    }

    delegate!(core, set_config(config: AppConfig) -> Model);
    delegate!(core, config() -> &AppConfig);
    delegate!(core, set_config_save_timeout(deadline: Instant) -> Model);
    delegate!(core, clear_config_save_timeout() -> Model);
    delegate!(core, reached_config_save_timeout() -> bool);
    delegate!(core, set_savefile_path(maybe_path: Option<impl Into<String>>) -> Model);
    delegate!(core, savefile_path() -> Option<&String>);
    delegate!(core, source(uuid: Uuid) -> AnyhowResult<&Source>);
    delegate!(core, sources_map() -> &HashMap<Uuid, Source>);
    delegate!(core, sources_list() -> Vec<&Source>);
    delegate!(core, add_source(source: Source) -> Result);
    delegate!(core, enable_source(uuid: Uuid) -> Result);
    delegate!(core, disable_source(uuid: Uuid) -> Result);
    delegate!(core, remove_source(uuid: Uuid) as remove_source_core -> Result);
    delegate!(core, clear_sources() as clear_sources_core -> Model);
    delegate!(core, source_loaders() ->
        &HashMap<Uuid, Rc<mpsc::Receiver<Result<Sample, libasampo::errors::Error>>>>);
    delegate!(core, handle_source_loader(messages: Vec<SourceLoaderMessage>)
        as handle_source_loader_core -> ());
    delegate!(core, remove_source_loader(uuid: Uuid) -> Result);
    delegate!(core, has_sources_loading() -> bool);
    delegate!(core, samples() -> std::cell::Ref<Vec<Sample>>);
    delegate!(core, set_selected_sample(s: Option<Sample>) -> Model);
    delegate!(core, selected_sample() -> Option<&Sample>);
    delegate!(core, sets_list() -> Vec<&SampleSet>);
    delegate!(core, sets_map() -> &HashMap<Uuid, SampleSet>);
    delegate!(core, get_set(uuid: Uuid) -> AnyhowResult<&SampleSet>);
    delegate!(core, add_set(set: SampleSet) -> Result);

    pub fn get_or_create_sampleset(
        model: AppModel,
        set_name: impl Into<String>,
    ) -> AnyhowResult<(AppModel, Uuid)> {
        let (result, uuid) = CoreModel::get_or_create_sampleset(model.core, set_name)?;

        Ok((
            AppModel {
                core: result,
                ..model
            },
            uuid,
        ))
    }

    #[cfg(test)]
    delegate!(core, remove_set(uuid: Uuid) -> Result);

    delegate!(core, clear_sets() as clear_sets_core -> Model);
    delegate!(core, add_to_set(sample: Sample, set_uuid: Uuid) -> Result);
    delegate!(core, get_set_most_recently_added_to() -> Option<Uuid>);
    delegate!(core, set_selected_set(maybe_uuid: Option<Uuid>) -> Result);
    delegate!(core, selected_set() -> Option<Uuid>);
    delegate!(core, set_labelling(set_uuid: Uuid, labelling: Option<SampleSetLabelling>) -> Result);
    delegate!(core, set_export_state(maybe_state: Option<ExportState>) -> Model);
    delegate!(core, export_state() -> Option<ExportState>);
    delegate!(core, set_export_job_rx(rx: Option<mpsc::Receiver<ExportJobMessage>>) -> Model);
    delegate!(core, export_job_rx() -> Option<Rc<mpsc::Receiver<ExportJobMessage>>>);

    delegate!(viewflags, set_are_sources_add_fs_fields_valid(valid: bool) -> Model);
    delegate!(viewflags, signal_sources_add_fs_begin_browse() -> Model);
    delegate!(viewflags, clear_signal_sources_add_fs_begin_browse() -> Model);
    delegate!(viewflags, signal_add_sample_to_set_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_add_sample_to_set_show_dialog() -> Model);
    delegate!(viewflags, enable_set_export() -> Model);
    delegate!(viewflags, disable_set_export() -> Model);
    delegate!(viewflags, is_set_export_enabled() -> bool);
    delegate!(viewflags, signal_add_set_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_add_set_show_dialog() -> Model);
    delegate!(viewflags, signal_export_begin_browse() -> Model);
    delegate!(viewflags, clear_signal_export_begin_browse() -> Model);
    delegate!(viewflags, signal_export_show_dialog() -> Model);
    delegate!(viewflags, clear_signal_export_show_dialog() -> Model);
    delegate!(viewflags, set_main_view_sensitive(sensitive: bool) -> Model);
    delegate!(viewflags, set_are_export_fields_valid(valid: bool) -> Model);
    delegate!(viewflags, is_main_view_sensitive() -> bool);
    delegate!(viewflags, are_export_fields_valid() -> bool);
    delegate!(viewflags, is_signalling_add_fs_source_begin_browse() -> bool);
    delegate!(viewflags, is_signalling_add_sample_to_set_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_add_set_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_export_show_dialog() -> bool);
    delegate!(viewflags, is_signalling_export_begin_browse() -> bool);
    delegate!(viewflags, are_add_fs_source_fields_valid() -> bool);
    delegate!(viewflags, enable_add_to_prev_set() -> Model);
    delegate!(viewflags, disable_add_to_prev_set() -> Model);
    delegate!(viewflags, is_add_to_prev_set_enabled() -> bool);

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
    delegate!(viewvalues, latency_approx_label_text() -> &String);
    delegate!(viewvalues, add_fs_source_name_entry_text() -> &String);
    delegate!(viewvalues, add_fs_source_path_entry_text() -> &String);
    delegate!(viewvalues, add_fs_source_extensions_entry_text() -> &String);
    delegate!(viewvalues, export_dialog_view() -> Option<&ExportDialogView>);
    delegate!(viewvalues, sources_sample_count() -> &HashMap<Uuid, usize>);
    delegate!(viewvalues, export_progress() -> Option<(usize, usize)>);
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
    delegate!(drum_machine, sequence() as drum_machine_sequence -> &DrumkitSequence);

    delegate!(drum_machine, set_sequence(sequence: DrumkitSequence)
        as set_drum_machine_sequence -> Model);

    delegate!(drum_machine, set_latest_event(event: Option<DrumkitSequenceEvent>)
        as set_latest_drum_machine_event -> Model);

    delegate!(drum_machine, poll_event()
        as drum_machine_poll_event -> Option<DrumkitSequenceEvent>);
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

        let model = model
            .add_set(SampleSet::BaseSampleSet(set.clone()))
            .unwrap();

        assert!(model.sets_map().contains_key(set.uuid()));
        assert_eq!(
            model.sets_map().get(set.uuid()).unwrap().name(),
            "Favorites"
        );

        let model = model.remove_set(*set.uuid()).unwrap();

        assert!(!model.sets_map().contains_key(set.uuid()));
    }
}
