// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::mpsc,
    thread::JoinHandle,
    time::Instant,
};

use anyhow::anyhow;
use gtk::prelude::ListModelExt;
use libasampo::{
    samples::{Sample, SampleOps},
    samplesets::{export::ExportJobMessage, SampleSet, SampleSetOps},
    sources::{Source, SourceOps},
};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    ext::{ClonedHashMapExt, ClonedVecExt},
    model::{DrumMachineModel, ModelResult, ViewFlags, ViewValues},
    view::samples::SampleListEntry,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportState {
    Exporting,
    Finished,
}

#[derive(Clone, Debug)]
pub struct AppModel {
    pub config: Option<AppConfig>,
    pub config_save_timeout: Option<std::time::Instant>,
    pub savefile: Option<String>,
    pub viewflags: ViewFlags,
    pub viewvalues: ViewValues,
    pub audiothread_tx: Option<mpsc::Sender<audiothread::Message>>,
    pub _audiothread_handle: Option<Rc<JoinHandle<()>>>,
    pub sources: HashMap<Uuid, Source>,
    pub sources_order: Vec<Uuid>,
    pub sources_loading:
        HashMap<Uuid, Rc<mpsc::Receiver<Result<Sample, libasampo::errors::Error>>>>,
    pub samples: Rc<RefCell<Vec<Sample>>>,
    pub samplelist_selected_sample: Option<Sample>,
    pub sets: HashMap<Uuid, SampleSet>,
    pub sets_order: Vec<Uuid>,
    pub sets_selected_set: Option<Uuid>,
    pub sets_most_recently_used_uuid: Option<Uuid>,
    pub sets_export_state: Option<ExportState>,
    pub sets_export_progress: Option<(usize, usize)>,
    pub export_job_rx: Option<Rc<mpsc::Receiver<ExportJobMessage>>>,
    pub drum_machine: DrumMachineModel,
}

pub type AppModelPtr = Rc<Cell<Option<AppModel>>>;

impl AppModel {
    pub fn new(
        config: Option<AppConfig>,
        savefile: Option<String>,
        audiothread_tx: Option<mpsc::Sender<audiothread::Message>>,
        audiothread_handle: Option<Rc<JoinHandle<()>>>,
    ) -> Self {
        let viewvalues = ViewValues::new(config.as_ref());

        let drum_machine = if let Some(tx) = &audiothread_tx {
            DrumMachineModel::new_with_render_thread(tx.clone())
        } else {
            DrumMachineModel::new(None, None)
        };

        AppModel {
            config,
            config_save_timeout: None,
            savefile,
            viewflags: ViewFlags::default(),
            viewvalues,
            audiothread_tx,
            _audiothread_handle: audiothread_handle,
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
            sets_export_progress: None,
            export_job_rx: None,
            drum_machine,
        }
    }

    pub fn disable_source(self, uuid: &Uuid) -> ModelResult {
        self.samples
            .borrow_mut()
            .retain(|s| s.source_uuid() != Some(uuid));

        Ok(AppModel {
            sources: self.sources.cloned_update_with(
                |mut s: HashMap<Uuid, Source>| -> Result<HashMap<Uuid, Source>, anyhow::Error> {
                    s.get_mut(uuid)
                        .ok_or_else(|| anyhow!("Failed to disable source: uuid not found!"))?
                        .disable();
                    Ok(s)
                },
            )?,
            ..self
        })
    }

    pub fn remove_source(self, uuid: &Uuid) -> ModelResult {
        let model = self.disable_source(uuid)?;

        Ok(AppModel {
            viewvalues: ViewValues {
                sources_sample_count: model
                    .viewvalues
                    .sources_sample_count
                    .clone_and_remove(uuid)?,
                ..model.viewvalues
            },
            sources_order: model.sources_order.clone_and_remove(uuid)?,
            sources: model.sources.clone_and_remove(uuid)?,
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

    pub fn add_sampleset(self, set: SampleSet) -> Self {
        AppModel {
            sets_order: self.sets_order.clone_and_push(*set.uuid()),
            sets: self.sets.clone_and_insert(*set.uuid(), set),
            ..self
        }
    }

    #[cfg(test)]
    pub fn remove_sampleset(self, uuid: &Uuid) -> ModelResult {
        Ok(AppModel {
            sets_order: self.sets_order.clone_and_remove(uuid)?,
            sets: self.sets.clone_and_remove(uuid)?,
            ..self
        })
    }
}

pub trait AppModelOps {
    fn set_config(self, config: AppConfig) -> AppModel;
    fn set_config_save_timeout(self, deadline: Instant) -> AppModel;
    fn clear_config_save_timeout(self) -> AppModel;

    fn add_source(self, source: Source) -> ModelResult;

    fn add_source_loader(
        self,
        source_uuid: Uuid,
        loader_rx: mpsc::Receiver<Result<Sample, libasampo::errors::Error>>,
    ) -> ModelResult;

    fn enable_source(self, uuid: &Uuid) -> ModelResult;
}

impl AppModelOps for AppModel {
    fn set_config(self, config: AppConfig) -> AppModel {
        AppModel {
            config: Some(config),
            ..self
        }
    }

    fn set_config_save_timeout(self, deadline: Instant) -> AppModel {
        AppModel {
            config_save_timeout: Some(deadline),
            ..self
        }
    }

    fn clear_config_save_timeout(self) -> AppModel {
        AppModel {
            config_save_timeout: None,
            ..self
        }
    }

    fn add_source(self, source: Source) -> ModelResult {
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

    fn add_source_loader(
        self,
        source_uuid: Uuid,
        loader_rx: mpsc::Receiver<Result<Sample, libasampo::errors::Error>>,
    ) -> ModelResult {
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

    fn enable_source(self, uuid: &Uuid) -> ModelResult {
        Ok(AppModel {
            sources: self.sources.cloned_update_with(
                |mut s: HashMap<Uuid, Source>| -> Result<HashMap<Uuid, Source>, anyhow::Error> {
                    s.get_mut(uuid)
                        .ok_or_else(|| anyhow!("Failed to enable source: UUID not present"))?
                        .enable();
                    Ok(s)
                },
            )?,
            ..self
        })
    }
}

#[cfg(test)]
mod tests {
    use libasampo::samplesets::BaseSampleSet;

    use super::*;

    #[test]
    fn test_add_remove_sampleset() {
        let model = AppModel::new(None, None, None, None);
        let set = BaseSampleSet::new("Favorites".to_string());

        let model = model.add_sampleset(SampleSet::BaseSampleSet(set.clone()));

        assert!(model.sets.contains_key(set.uuid()));
        assert_eq!(model.sets.get(set.uuid()).unwrap().name(), "Favorites");

        let model = model.remove_sampleset(set.uuid()).unwrap();

        assert!(!model.sets.contains_key(set.uuid()));
    }
}
