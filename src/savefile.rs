// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{io::Write, path::Path};

use anyhow::anyhow;
use libasampo::{
    errors::Error as LaError,
    samplesets::SampleSet as DomSampleSet,
    serialize::{SampleSet as SerSampleSet, Source as SerSource, TryFromDomain, TryIntoDomain},
    sources::Source as DomSource,
};
use serde::{Deserialize, Serialize};

use crate::model::AppModel;

type AnyhowResult<T> = Result<T, anyhow::Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavefileV1 {
    sources: Vec<SerSource>,
    samplesets: Vec<SerSampleSet>,
}

impl SavefileV1 {
    pub fn from_appmodel(model: &AppModel) -> AnyhowResult<SavefileV1> {
        Ok(SavefileV1 {
            sources: model
                .sources_order
                .iter()
                .map(|uuid| SerSource::try_from_domain(model.sources.get(uuid).unwrap()))
                .collect::<Result<Vec<SerSource>, LaError>>()?,

            samplesets: model
                .sets_order
                .iter()
                .map(|uuid| SerSampleSet::try_from_domain(model.sets.get(uuid).unwrap()))
                .collect::<Result<Vec<SerSampleSet>, LaError>>()?,
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
            Savefile::V1(source) => source.sources_domained(),
        }
    }

    pub fn sets_domained(&self) -> AnyhowResult<Vec<DomSampleSet>> {
        match self {
            Savefile::V1(source) => source.sets_domained(),
        }
    }
}
