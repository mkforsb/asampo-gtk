// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::mpsc::{self, Sender},
    thread::JoinHandle,
};

use anyhow::anyhow;
use gtk::{gio::ListStore, prelude::*};
use libasampo::{prelude::*, samples::Sample, samplesets::SampleSet, sources::Source};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    ext::{ClonedHashMapExt, ClonedVecExt},
    view::{dialogs, samples::SampleListEntry},
};

#[derive(Debug, Clone)]
pub struct ViewFlags {
    pub sources_add_fs_fields_valid: bool,
    pub sources_add_fs_begin_browse: bool,
    pub samples_sidebar_add_to_set_show_dialog: bool,
    pub samples_sidebar_add_to_prev_enabled: bool,
    pub samplesets_add_fields_valid: bool,
    pub samplesets_export_enabled: bool,
    pub samplesets_export_show_dialog: bool,
    pub samplesets_export_begin_browse: bool,
    pub samplesets_export_fields_valid: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ViewFlags {
    fn default() -> Self {
        ViewFlags {
            sources_add_fs_fields_valid: false,
            sources_add_fs_begin_browse: false,
            samples_sidebar_add_to_set_show_dialog: false,
            samples_sidebar_add_to_prev_enabled: false,
            samplesets_add_fields_valid: false,
            samplesets_export_enabled: false,
            samplesets_export_show_dialog: false,
            samplesets_export_begin_browse: false,
            samplesets_export_fields_valid: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExportKind {
    PlainCopy,
    Conversion,
}

#[derive(Debug, Clone)]
pub struct ViewValues {
    pub sources_add_fs_name_entry: String,
    pub sources_add_fs_path_entry: String,
    pub sources_add_fs_extensions_entry: String,
    pub samples_list_filter: String,
    pub settings_latency_approx_label: String,
    pub samples_listview_model: ListStore,
    pub samples_selected_sample: Option<Sample>,
    pub samples_set_most_recently_used: Option<Uuid>,
    pub samplesets_add_name_entry: String,
    pub samplesets_selected_set: Option<Uuid>,
    pub samplesets_export_dialog_view: Option<dialogs::ExportDialogView>,
    pub samplesets_export_target_dir_entry: String,
    pub samplesets_export_kind: Option<ExportKind>,
}

impl Default for ViewValues {
    fn default() -> Self {
        ViewValues {
            sources_add_fs_name_entry: String::default(),
            sources_add_fs_path_entry: String::default(),
            sources_add_fs_extensions_entry: String::default(),
            samples_list_filter: String::default(),
            settings_latency_approx_label: String::default(),
            samples_listview_model: ListStore::new::<SampleListEntry>(),
            samples_selected_sample: None,
            samples_set_most_recently_used: None,
            samplesets_add_name_entry: String::default(),
            samplesets_selected_set: None,
            samplesets_export_dialog_view: None,
            samplesets_export_target_dir_entry: String::default(),
            samplesets_export_kind: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppModel {
    pub config: Option<AppConfig>,
    pub config_save_timeout: Option<u32>,
    pub savefile: Option<String>,
    pub viewflags: ViewFlags,
    pub viewvalues: ViewValues,
    pub audiothread_tx: Option<Sender<audiothread::Message>>,
    pub _audiothread_handle: Option<Rc<JoinHandle<()>>>,
    pub sources: HashMap<Uuid, Source>,
    pub sources_order: Vec<Uuid>,
    pub samples: Rc<RefCell<Vec<Sample>>>,
    pub samplesets: HashMap<Uuid, SampleSet>,
    pub samplesets_order: Vec<Uuid>,
}

pub type AppModelPtr = Rc<Cell<Option<AppModel>>>;

impl AppModel {
    pub fn new(
        config: Option<AppConfig>,
        savefile: Option<String>,
        tx: Option<mpsc::Sender<audiothread::Message>>,
        handle: Option<Rc<JoinHandle<()>>>,
    ) -> Self {
        let settings_latency_approx_label = match &config {
            Some(conf) => conf.fmt_latency_approx(),
            None => "???".to_string(),
        };

        AppModel {
            config,
            config_save_timeout: None,
            savefile,
            viewflags: ViewFlags::default(),
            viewvalues: ViewValues {
                settings_latency_approx_label,
                samples_listview_model: ListStore::new::<SampleListEntry>(),
                ..ViewValues::default()
            },
            audiothread_tx: tx,
            _audiothread_handle: handle,
            sources: HashMap::new(),
            sources_order: Vec::new(),
            samples: Rc::new(RefCell::new(Vec::new())),
            samplesets: HashMap::new(),
            samplesets_order: Vec::new(),
        }
    }

    pub fn add_source(self, source: Source) -> Self {
        let mut new_sources_order = self.sources_order.clone();
        new_sources_order.push(*source.uuid());

        let mut new_sources = self.sources.clone();
        new_sources.insert(*source.uuid(), source);

        AppModel {
            sources_order: new_sources_order,
            sources: new_sources,
            ..self
        }
    }

    pub fn load_enabled_sources(&self) -> Result<(), anyhow::Error> {
        for uuid in self.sources_order.iter() {
            if self
                .sources
                .get(uuid)
                .ok_or(anyhow::anyhow!(
                    "Failed to load source: reference to nonexistant uuid"
                ))?
                .is_enabled()
            {
                self.samples
                    .borrow_mut()
                    .extend(self.sources.get(uuid).unwrap().list()?);
            }
        }
        Ok(())
    }

    pub fn enable_source(self, uuid: &Uuid) -> Result<Self, anyhow::Error> {
        self.samples.borrow_mut().extend(
            self.sources
                .get(uuid)
                .ok_or_else(|| anyhow!("Failed to enable source: uuid not found!"))?
                .list()?,
        );

        Ok(AppModel {
            sources: self.sources.cloned_update_with(
                |mut s: HashMap<Uuid, Source>| -> Result<HashMap<Uuid, Source>, anyhow::Error> {
                    s.get_mut(uuid)
                        .ok_or_else(|| anyhow!("Failed to enable source: uuid not found!"))?
                        .enable();
                    Ok(s)
                },
            )?,
            ..self
        })
    }

    pub fn disable_source(self, uuid: &Uuid) -> Result<Self, anyhow::Error> {
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

    pub fn remove_source(self, uuid: &Uuid) -> Result<Self, anyhow::Error> {
        Ok(AppModel {
            sources_order: self.sources_order.clone_and_remove(uuid)?,
            sources: self.sources.clone_and_remove(uuid)?,
            ..self.disable_source(uuid)?
        })
    }

    pub fn map<F: FnOnce(Self) -> Self>(self, f: F) -> Self {
        f(self)
    }

    pub fn map_ref<F: FnOnce(&Self)>(self, f: F) -> Self {
        f(&self);
        self
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
            "showing {} samples",
            self.viewvalues.samples_listview_model.n_items()
        );
    }

    pub fn add_sampleset(self, set: SampleSet) -> Self {
        AppModel {
            samplesets_order: self.samplesets_order.clone_and_push(*set.uuid()),
            samplesets: self.samplesets.clone_and_insert(*set.uuid(), set),
            ..self
        }
    }

    #[cfg(test)]
    pub fn remove_sampleset(self, uuid: &Uuid) -> Result<Self, anyhow::Error> {
        Ok(AppModel {
            samplesets_order: self.samplesets_order.clone_and_remove(uuid)?,
            samplesets: self.samplesets.clone_and_remove(uuid)?,
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

        assert!(model.samplesets.contains_key(set.uuid()));
        assert_eq!(
            model.samplesets.get(set.uuid()).unwrap().name(),
            "Favorites"
        );

        let model = model.remove_sampleset(set.uuid()).unwrap();

        assert!(!model.samplesets.contains_key(set.uuid()));
    }
}
