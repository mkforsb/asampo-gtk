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
use libasampo::{prelude::*, samples::Sample, sources::Source};
use uuid::Uuid;

use crate::{config::AppConfig, ext::{ClonedHashMapExt, ClonedVecExt}, view::samples::SampleListEntry};

#[derive(Debug, Clone)]
pub struct ViewFlags {
    pub sources_add_fs_fields_valid: bool,
    pub sources_add_fs_browse: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ViewFlags {
    fn default() -> Self {
        ViewFlags {
            sources_add_fs_fields_valid: false,
            sources_add_fs_browse: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViewValues {
    pub sources_add_fs_name_entry: String,
    pub sources_add_fs_path_entry: String,
    pub sources_add_fs_extensions_entry: String,
    pub samples_list_filter: String,
    pub settings_latency_approx_label: String,
    pub samples_listview_model: ListStore,
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
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppModel {
    pub timer_enabled: bool,
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
            timer_enabled: true,
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

    pub fn enable_source(self, uuid: Uuid) -> Result<Self, anyhow::Error> {
        self.samples.borrow_mut().extend(
            self.sources
                .get(&uuid)
                .ok_or_else(|| anyhow!("Failed to enable source: uuid not found!"))?
                .list()?,
        );

        Ok(AppModel {
            sources: self.sources.cloned_update_with(
                |mut s: HashMap<Uuid, Source>| -> Result<HashMap<Uuid, Source>, anyhow::Error> {
                    s.get_mut(&uuid)
                        .ok_or_else(|| anyhow!("Failed to enable source: uuid not found!"))?
                        .enable();
                    Ok(s)
                },
            )?,
            ..self
        })
    }

    pub fn disable_source(self, uuid: Uuid) -> Result<Self, anyhow::Error> {
        self.samples
            .borrow_mut()
            .retain(|s| s.source_uuid() != Some(&uuid));

        Ok(AppModel {
            sources: self.sources.cloned_update_with(
                |mut s: HashMap<Uuid, Source>| -> Result<HashMap<Uuid, Source>, anyhow::Error> {
                    s.get_mut(&uuid)
                        .ok_or_else(|| anyhow!("Failed to disable source: uuid not found!"))?
                        .disable();
                    Ok(s)
                },
            )?,
            ..self
        })
    }

    pub fn remove_source(self, uuid: Uuid) -> Result<Self, anyhow::Error> {
        Ok(AppModel {
            sources_order: self.sources_order.clone_and_remove(&uuid)?,
            sources: self.sources.clone_and_remove(&uuid)?,
            ..self.disable_source(uuid)?
        })
    }

    // pub fn map<F: FnOnce(Self) -> Self>(self, f: F) -> Self {
    //     f(self)
    // }

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
            let fragments = filter.split(' ').map(|s| s.to_string()).collect::<Vec<_>>();

            let mut samples = self.samples.borrow().clone();
            samples.retain(|x| fragments.iter().all(|frag| x.uri().contains(frag)));

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
}
