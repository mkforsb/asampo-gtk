// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::sync::mpsc;

use anyhow::anyhow;
use gtk::glib::clone;
use libasampo::{
    samples::Sample,
    sources::{file_system_source::FilesystemSource, Source, SourceOps},
};

mod app;
pub(in crate::model) mod delegate;
mod drum_machine;
mod view;

pub mod util;

pub use app::{AppModel, AppModelPtr, ExportState};
pub use drum_machine::DrumMachineModel;
pub use view::{ExportKind, ViewFlags, ViewValues};

pub fn sources_add_fs_fields_valid(model: &AppModel) -> bool {
    !(model.viewvalues.sources_add_fs_name_entry.is_empty()
        || model.viewvalues.sources_add_fs_path_entry.is_empty()
        || model.viewvalues.sources_add_fs_extensions_entry.is_empty())
}

pub type ModelResult = Result<AppModel, anyhow::Error>;

pub trait ModelOps {
    fn validate_sources_add_fs_fields(self) -> AppModel;

    fn commit_file_system_source(self) -> Result<AppModel, anyhow::Error>;

    fn add_file_system_source(
        self,
        name: String,
        path: String,
        exts: Vec<String>,
    ) -> Result<AppModel, anyhow::Error>;

    fn tap<F: FnOnce(&AppModel)>(self, f: F) -> AppModel;
}

impl ModelOps for AppModel {
    fn validate_sources_add_fs_fields(self) -> AppModel {
        let valid = sources_add_fs_fields_valid(&self);
        self.set_is_sources_add_fs_fields_valid(valid)
    }

    fn commit_file_system_source(self) -> Result<AppModel, anyhow::Error> {
        if sources_add_fs_fields_valid(&self) {
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
    fn add_file_system_source(
        self,
        name: String,
        path: String,
        exts: Vec<String>,
    ) -> Result<AppModel, anyhow::Error> {
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
            .set_is_sources_add_fs_fields_valid(false)
            .add_source_loader(uuid, loader_rx)
    }

    fn tap<F: FnOnce(&AppModel)>(self, f: F) -> AppModel {
        f(&self);
        self
    }
}
