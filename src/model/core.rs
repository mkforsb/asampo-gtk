// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::mpsc, time::Instant};

use anyhow::anyhow;
use libasampo::{
    samples::{Sample, SampleOps},
    samplesets::{export::ExportJobMessage, BaseSampleSet, DrumkitLabel, SampleSet, SampleSetOps},
    sequences::{DrumkitSequence, StepSequenceOps},
    sources::{Source, SourceOps},
};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    ext::{ClonedHashMapExt, ClonedVecExt},
    model::AnyhowResult,
};

pub type SourceLoadMsg = Result<Sample, libasampo::errors::Error>;

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
    sources_loading: HashMap<Uuid, Rc<mpsc::Receiver<SourceLoadMsg>>>,
    samples: Rc<RefCell<Vec<Sample>>>,
    samplelist_selected_sample: Option<Sample>,
    sets: HashMap<Uuid, SampleSet>,
    sets_order: Vec<Uuid>,
    sets_selected_set: Option<Uuid>,
    sets_selected_sample: Option<Sample>,
    sets_most_recently_used_uuid: Option<Uuid>,
    sets_export_state: Option<ExportState>,
    sequences: HashMap<Uuid, DrumkitSequence>,
    sequences_order: Vec<Uuid>,
    sequences_selected_sequence: Option<Uuid>,
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
            sets_selected_sample: None,
            sets_most_recently_used_uuid: None,
            sets_export_state: None,
            sequences: HashMap::new(),
            sequences_order: Vec::new(),
            sequences_selected_sequence: None,
            export_job_rx: None,
        }
    }

    pub fn is_modified_vs(&self, other: &Self) -> bool {
        self.config != other.config
            || self.savefile != other.savefile
            || self.sources != other.sources
            || self.sources_order != other.sources_order
            || self.sets != other.sets
            || self.sets_order != other.sets_order
            || self.sequences != other.sequences
            || self.sequences_order != other.sequences_order
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

    fn spawn_source_loader(source: Source) -> mpsc::Receiver<SourceLoadMsg> {
        let (tx, rx) = mpsc::channel::<SourceLoadMsg>();

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
        loader_rx: mpsc::Receiver<SourceLoadMsg>,
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

    pub fn handle_source_loader(&self, messages: Vec<SourceLoadMsg>) {
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

    pub fn set(&self, uuid: Uuid) -> AnyhowResult<&SampleSet> {
        self.sets
            .get(&uuid)
            .ok_or(anyhow!("Failed to fetch sample set: UUID not present"))
    }

    fn set_mut(&mut self, uuid: Uuid) -> AnyhowResult<&mut SampleSet> {
        self.sets
            .get_mut(&uuid)
            .ok_or(anyhow!("Failed to fetch sample set: UUID not present"))
    }

    pub fn add_set(self, set: SampleSet) -> AnyhowResult<CoreModel> {
        if self.sets.contains_key(&set.uuid()) {
            Err(anyhow!("Failed to add set: UUID in use"))
        } else {
            let uuid = set.uuid();

            Ok(CoreModel {
                sets: self.sets.clone_and_insert(uuid, set),
                sets_order: self.sets_order.clone_and_push(uuid),
                ..self
            })
        }
    }

    pub fn insert_set(self, set: SampleSet, position: usize) -> AnyhowResult<CoreModel> {
        if self.sets.contains_key(&set.uuid()) {
            Err(anyhow!("Failed to insert sample set: UUID in use"))
        } else {
            let uuid = set.uuid();

            Ok(CoreModel {
                sets: self.sets.clone_and_insert(uuid, set),
                sets_order: self.sets_order.clone_and_insert(uuid, position),
                ..self
            })
        }
    }

    pub fn get_or_create_set(
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
                let new_uuid = new_set.uuid();

                Ok((model.add_set(new_set)?, new_uuid))
            }
        }
    }

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

        result.set_mut(set_uuid)?.add(
            self.source(
                *sample
                    .source_uuid()
                    .ok_or(anyhow!("Sample missing source UUID"))?,
            )?,
            sample,
        )?;

        result.set_set_most_recently_added_to(Some(set_uuid))
    }

    pub fn remove_from_set(self, sample: &Sample, set_uuid: Uuid) -> AnyhowResult<CoreModel> {
        let mut result = self.clone();

        result
            .set_mut(set_uuid)?
            .remove(sample)
            .map_err(|e| anyhow!("Could not remove sample: {e}"))?;

        Ok(result)
    }

    // TODO: use "sset" for referring to sample sets?
    pub fn set_sample_label(
        self,
        set_uuid: Uuid,
        sample: Sample,
        label: Option<DrumkitLabel>,
    ) -> AnyhowResult<CoreModel> {
        let mut result = self.clone();

        result
            .set_mut(set_uuid)?
            .set_label::<DrumkitLabel, Option<DrumkitLabel>>(&sample, label)?;

        Ok(result)
    }

    pub fn set_set_most_recently_added_to(
        self,
        maybe_uuid: Option<Uuid>,
    ) -> AnyhowResult<CoreModel> {
        match maybe_uuid.and_then(|uuid| self.set(uuid).err()) {
            Some(err) => Err(err),
            None => Ok(CoreModel {
                sets_most_recently_used_uuid: maybe_uuid,
                ..self
            }),
        }
    }

    pub fn set_most_recently_added_to(&self) -> Option<Uuid> {
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

    pub fn set_selected_set_member(self, maybe_sample: Option<Sample>) -> CoreModel {
        CoreModel {
            sets_selected_sample: maybe_sample,
            ..self
        }
    }

    pub fn selected_set_member(&self) -> Option<&Sample> {
        self.sets_selected_sample.as_ref()
    }

    pub fn selected_set(&self) -> Option<Uuid> {
        self.sets_selected_set
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

    pub fn sequence(&self, uuid: Uuid) -> AnyhowResult<&DrumkitSequence> {
        self.sequences
            .get(&uuid)
            .ok_or(anyhow!("Failed to get sequence: UUID not present"))
    }

    pub fn sequences_list(&self) -> Vec<&DrumkitSequence> {
        self.sequences_order
            .iter()
            .map(|uuid| self.sequence(*uuid).unwrap())
            .collect()
    }

    pub fn sequences_map(&self) -> &HashMap<Uuid, DrumkitSequence> {
        &self.sequences
    }

    pub fn add_sequence(self, sequence: DrumkitSequence) -> AnyhowResult<CoreModel> {
        if self.sequences.contains_key(&sequence.uuid()) {
            Err(anyhow!("Failed to add sequence: UUID in use"))
        } else {
            let uuid = sequence.uuid();

            Ok(CoreModel {
                sequences: self.sequences.clone_and_insert(uuid, sequence),
                sequences_order: self.sequences_order.clone_and_push(uuid),
                ..self
            })
        }
    }

    pub fn insert_sequence(
        self,
        sequence: DrumkitSequence,
        position: usize,
    ) -> AnyhowResult<CoreModel> {
        if self.sequences.contains_key(&sequence.uuid()) {
            Err(anyhow!("Failed to insert sequence: UUID in use"))
        } else {
            let uuid = sequence.uuid();

            Ok(CoreModel {
                sequences: self.sequences.clone_and_insert(uuid, sequence),
                sequences_order: self.sequences_order.clone_and_insert(uuid, position),
                ..self
            })
        }
    }

    pub fn set_selected_sequence(self, maybe_uuid: Option<Uuid>) -> AnyhowResult<CoreModel> {
        if let Some(false) = maybe_uuid.map(|uuid| self.sequences.contains_key(&uuid)) {
            Err(anyhow!("Failed to set selected sequence: UUID not present"))
        } else {
            Ok(CoreModel {
                sequences_selected_sequence: maybe_uuid,
                ..self
            })
        }
    }

    pub fn selected_sequence(&self) -> Option<Uuid> {
        self.sequences_selected_sequence
    }

    pub fn remove_sequence(self, uuid: Uuid) -> AnyhowResult<CoreModel> {
        Ok(CoreModel {
            sequences_order: self.sequences_order.clone_and_remove(&uuid)?,
            sequences: self.sequences.clone_and_remove(&uuid)?,
            ..self
        })
    }

    pub fn clear_sequences(self) -> CoreModel {
        CoreModel {
            sequences: HashMap::new(),
            sequences_order: Vec::new(),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use libasampo::samplesets::BaseSampleSet;

    use super::*;

    #[test]
    fn test_add_remove_sampleset() {
        let model = CoreModel::new(AppConfig::default(), Some("/tmp/config.json"));
        let set = BaseSampleSet::new("Favorites".to_string());

        let model = model
            .add_set(SampleSet::BaseSampleSet(set.clone()))
            .unwrap();

        assert!(model.sets_map().contains_key(&set.uuid()));
        assert_eq!(
            model.sets_map().get(&set.uuid()).unwrap().name(),
            "Favorites"
        );

        let model = model.remove_set(set.uuid()).unwrap();

        assert!(!model.sets_map().contains_key(&set.uuid()));
    }
}
