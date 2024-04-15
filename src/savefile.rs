// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{io::Write, path::Path};

use libasampo::{prelude::*, serialize::IntoDomain};
use serde::{Deserialize, Serialize};

use crate::model::AppModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavefileV1 {
    sources: Vec<libasampo::serialize::Source>,
}

impl SavefileV1 {
    pub fn into_appmodel(self) -> Result<AppModel, anyhow::Error> {
        let mut model = AppModel::new(None, None, None);

        for src in self.sources {
            let source = src.into_domain();

            model.sources_order.push(*source.uuid());
            model.sources.insert(*source.uuid(), source);
        }

        Ok(model)
    }

    pub fn from_appmodel(model: &AppModel) -> SavefileV1 {
        SavefileV1 {
            sources: model
                .sources_order
                .iter()
                .map(|uuid| model.sources.get(uuid).unwrap().clone().into())
                .collect::<Vec<libasampo::serialize::Source>>(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Savefile {
    V1(SavefileV1),
}

impl Savefile {
    pub fn save(model: &AppModel, filename: &str) -> Result<(), anyhow::Error> {
        let json = serde_json::to_string(&Savefile::V1(SavefileV1::from_appmodel(model)))?;

        {
            if let Some(path) = Path::new(filename).parent() {
                std::fs::create_dir_all(path)?;
            }

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
