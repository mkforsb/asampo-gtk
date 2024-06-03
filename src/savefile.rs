// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{io::Write, path::Path};

use libasampo::{
    self as la,
    prelude::*,
    serialize::{TryFromDomain, TryIntoDomain},
};
use serde::{Deserialize, Serialize};

use crate::model::AppModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavefileV1 {
    sources: Vec<la::serialize::Source>,
    samplesets: Vec<la::serialize::SampleSet>,
}

impl SavefileV1 {
    pub fn into_appmodel(self) -> Result<AppModel, anyhow::Error> {
        let mut model = AppModel::new(None, None, None, None);

        for src in self.sources {
            let source = src.try_into_domain()?;

            model.sources_order.push(*source.uuid());
            model.sources.insert(*source.uuid(), source);
        }

        for set in self.samplesets {
            let sampleset = set.try_into_domain()?;

            model.sets_order.push(*sampleset.uuid());
            model.sets.insert(*sampleset.uuid(), sampleset);
        }

        Ok(model)
    }

    pub fn from_appmodel(model: &AppModel) -> Result<SavefileV1, anyhow::Error> {
        Ok(SavefileV1 {
            sources: model
                .sources_order
                .iter()
                .map(|uuid| {
                    la::serialize::Source::try_from_domain(model.sources.get(uuid).unwrap())
                })
                .collect::<Result<Vec<la::serialize::Source>, la::errors::Error>>()?,

            samplesets: model
                .sets_order
                .iter()
                .map(|uuid| {
                    la::serialize::SampleSet::try_from_domain(model.sets.get(uuid).unwrap())
                })
                .collect::<Result<Vec<la::serialize::SampleSet>, la::errors::Error>>()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Savefile {
    V1(SavefileV1),
}

impl Savefile {
    pub fn save(model: &AppModel, filename: &str) -> Result<(), anyhow::Error> {
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

    pub fn load(filename: &str) -> Result<AppModel, anyhow::Error> {
        match serde_json::from_str::<Savefile>(&String::from_utf8(std::fs::read(filename)?)?)? {
            Savefile::V1(sav) => Ok(AppModel {
                savefile: Some(filename.to_string()),
                ..sav.into_appmodel()?
            }),
        }
    }
}
