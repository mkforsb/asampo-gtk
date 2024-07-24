// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{io::Write, path::Path};

use anyhow::anyhow;
use libasampo::{
    errors::Error as LaError,
    samplesets::{SampleSet as DomSampleSet, SampleSetOps},
    sequences::{DrumkitSequence, StepSequenceOps},
    serialize::{
        SampleSet as SerSampleSet, Sequence as SerSequence, Source as SerSource, TryFromDomain,
        TryIntoDomain,
    },
    sources::Source as DomSource,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::AppModel;

type AnyhowResult<T> = Result<T, anyhow::Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavefileV1 {
    sources: Vec<SerSource>,
    samplesets: Vec<SerSampleSet>,
    sequences: Vec<SerSequence>,
    drum_machine_sequence: SerSequence,
    drum_machine_loaded_sequence: Option<Uuid>,
    drum_machine_sampleset: SerSampleSet,
    drum_machine_loaded_sampleset: Option<Uuid>,
}

impl SavefileV1 {
    pub fn from_appmodel(model: &AppModel) -> AnyhowResult<SavefileV1> {
        Ok(SavefileV1 {
            sources: model
                .sources_list()
                .iter()
                .map(|source| SerSource::try_from_domain(source))
                .collect::<Result<Vec<SerSource>, LaError>>()?,

            samplesets: model
                .sets_list()
                .iter()
                .map(|set| SerSampleSet::try_from_domain(set))
                .collect::<Result<Vec<SerSampleSet>, LaError>>()?,

            sequences: model
                .sequences_list()
                .iter()
                .map(|seq| SerSequence::try_from_domain(seq))
                .collect::<Result<Vec<SerSequence>, LaError>>()?,

            drum_machine_sequence: SerSequence::try_from_domain(model.drum_machine_sequence())?,
            drum_machine_loaded_sequence: model.drum_machine_loaded_sequence().map(|s| s.uuid()),
            drum_machine_sampleset: SerSampleSet::try_from_domain(model.drum_machine_sampleset())?,
            drum_machine_loaded_sampleset: model.drum_machine_loaded_sampleset().map(|s| s.uuid()),
        })
    }

    pub fn sources_domained(&self) -> AnyhowResult<Vec<DomSource>> {
        self.sources
            .iter()
            .map(|s| {
                s.clone()
                    .try_into_domain()
                    .map_err(|e| anyhow!("Failed to deserialize source: {e}"))
            })
            .collect()
    }

    pub fn sets_domained(&self) -> AnyhowResult<Vec<DomSampleSet>> {
        self.samplesets
            .iter()
            .map(|s| {
                s.clone()
                    .try_into_domain()
                    .map_err(|e| anyhow!("Failed to deserialize sample set: {e}"))
            })
            .collect()
    }

    pub fn sequences_domained(&self) -> AnyhowResult<Vec<DrumkitSequence>> {
        self.sequences
            .iter()
            .map(|s| {
                s.clone()
                    .try_into_domain()
                    .map_err(|e| anyhow!("Failed to deserialize sequence: {e}"))
            })
            .collect()
    }

    pub fn drum_machine_sequence_domained(&self) -> AnyhowResult<DrumkitSequence> {
        Ok(self.drum_machine_sequence.clone().try_into_domain()?)
    }

    pub fn drum_machine_loaded_sequence(&self) -> Option<Uuid> {
        self.drum_machine_loaded_sequence
    }

    pub fn drum_machine_sampleset_domained(&self) -> AnyhowResult<DomSampleSet> {
        Ok(self.drum_machine_sampleset.clone().try_into_domain()?)
    }

    pub fn drum_machine_loaded_sampleset(&self) -> Option<Uuid> {
        self.drum_machine_loaded_sampleset
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Savefile {
    V1(SavefileV1),
}

impl Savefile {
    pub fn save(model: &AppModel, filename: &str) -> AnyhowResult<()> {
        let json = serde_json::to_string_pretty(&Savefile::V1(SavefileV1::from_appmodel(model)?))?;

        if let Some(path) = Path::new(filename).parent() {
            std::fs::create_dir_all(path)?;
        }

        {
            let mut fd = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(filename)?;

            fd.write_all(json.as_bytes())?;
        }

        Ok(())
    }

    pub fn load(filename: &str) -> AnyhowResult<Savefile> {
        Ok(serde_json::from_str::<Savefile>(&String::from_utf8(
            std::fs::read(filename)?,
        )?)?)
    }

    pub fn sources_domained(&self) -> AnyhowResult<Vec<DomSource>> {
        match self {
            Savefile::V1(sf) => sf.sources_domained(),
        }
    }

    pub fn sets_domained(&self) -> AnyhowResult<Vec<DomSampleSet>> {
        match self {
            Savefile::V1(sf) => sf.sets_domained(),
        }
    }

    pub fn sequences_domained(&self) -> AnyhowResult<Vec<DrumkitSequence>> {
        match self {
            Savefile::V1(sf) => sf.sequences_domained(),
        }
    }

    pub fn drum_machine_sequence_domained(&self) -> AnyhowResult<DrumkitSequence> {
        match self {
            Savefile::V1(sf) => sf.drum_machine_sequence_domained(),
        }
    }

    pub fn drum_machine_loaded_sequence(&self) -> Option<Uuid> {
        match self {
            Savefile::V1(sf) => sf.drum_machine_loaded_sequence(),
        }
    }

    pub fn drum_machine_sampleset_domained(&self) -> AnyhowResult<DomSampleSet> {
        match self {
            Savefile::V1(sf) => sf.drum_machine_sampleset_domained(),
        }
    }

    pub fn drum_machine_loaded_sampleset(&self) -> Option<Uuid> {
        match self {
            Savefile::V1(sf) => sf.drum_machine_loaded_sampleset(),
        }
    }
}
