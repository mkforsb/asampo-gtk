// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use anyhow::anyhow;

use libasampo::{
    samples::SampleOps,
    samplesets::{BaseSampleSet, SampleSet, SampleSetOps},
};
use uuid::Uuid;

use crate::model::{AppModel, ViewFlags, ViewValues};

pub fn get_or_create_sampleset(
    model: AppModel,
    name: String,
) -> Result<(AppModel, Uuid), anyhow::Error> {
    match model
        .samplesets
        .iter()
        .find(|(_, set)| set.name() == name)
        .map(|(uuid, _)| *uuid)
    {
        Some(uuid) => Ok((model, uuid)),
        None => {
            let new_set = SampleSet::BaseSampleSet(BaseSampleSet::new(name));
            let new_uuid = *new_set.uuid();

            Ok((model.add_sampleset(new_set), new_uuid))
        }
    }
}

pub fn add_selected_sample_to_sampleset_by_uuid(
    model: AppModel,
    uuid: &Uuid,
) -> Result<AppModel, anyhow::Error> {
    let sample = model
        .viewvalues
        .samples_selected_sample
        .as_ref()
        .ok_or(anyhow!("No selected sample"))?;

    let source = model
        .sources
        .get(
            sample
                .source_uuid()
                .ok_or(anyhow!("Selected sample has no source"))?,
        )
        .ok_or(anyhow!("Could not obtain source for selected sample"))?;

    let mut model = model.clone();

    model
        .samplesets
        .get_mut(uuid)
        .ok_or(anyhow!("Sample set not found (by uuid)"))?
        .add(source, sample.clone())?;

    Ok(AppModel {
        viewflags: ViewFlags {
            samples_sidebar_add_to_prev_enabled: true,
            ..model.viewflags
        },
        viewvalues: ViewValues {
            samples_set_most_recently_used: Some(*uuid),
            ..model.viewvalues
        },
        ..model
    })
}

pub fn check_sources_add_fs_valid(model: AppModel) -> AppModel {
    AppModel {
        viewflags: ViewFlags {
            sources_add_fs_fields_valid: !model.viewvalues.sources_add_fs_name_entry.is_empty()
                && !model.viewvalues.sources_add_fs_path_entry.is_empty()
                && !model.viewvalues.sources_add_fs_extensions_entry.is_empty(),
            ..model.viewflags
        },
        ..model
    }
}
