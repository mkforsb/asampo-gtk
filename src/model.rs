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

use crate::ext::ClonedUpdateWith;
use crate::samples::SampleListEntry;

#[derive(Debug, Clone)]
pub struct AppFlags {
    pub sources_add_fs_fields_valid: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for AppFlags {
    fn default() -> Self {
        AppFlags {
            sources_add_fs_fields_valid: false,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AppValues {
    pub sources_add_fs_name_entry: String,
    pub sources_add_fs_path_entry: String,
    pub sources_add_fs_extensions_entry: String,
    pub samples_list_filter: String,
}

#[derive(Clone, Debug)]
pub struct AppModel {
    pub flags: AppFlags,
    pub values: AppValues,
    pub audiothread_tx: Sender<audiothread::Message>,
    pub _audiothread_handle: Rc<JoinHandle<()>>,
    pub sources: HashMap<Uuid, Source>,
    pub sources_order: Vec<Uuid>,
    pub samples: Rc<RefCell<Vec<Sample>>>,
    pub samples_listview_model: ListStore,
}

pub type AppModelPtr = Rc<Cell<Option<AppModel>>>;

impl AppModel {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        AppModel {
            flags: AppFlags::default(),
            values: AppValues::default(),
            audiothread_tx: tx,
            _audiothread_handle: Rc::new(audiothread::spawn(
                rx,
                Some(audiothread::Opts::default().with_bufsize_n_stereo_samples(1024)),
            )),
            sources: HashMap::new(),
            sources_order: Vec::new(),
            samples: Rc::new(RefCell::new(Vec::new())),
            samples_listview_model: ListStore::new::<crate::samples::SampleListEntry>(),
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

    pub fn populate_samples_listmodel(self) -> Self {
        let filter = &self.values.samples_list_filter;
        self.samples_listview_model.remove_all();

        if filter.is_empty() {
            let samples = self
                .samples
                .borrow()
                .iter()
                .map(|s| SampleListEntry::new(s.clone()))
                .collect::<Vec<_>>();

            self.samples_listview_model
                .extend_from_slice(samples.as_slice());
        } else {
            let fragments = filter.split(' ').map(|s| s.to_string()).collect::<Vec<_>>();

            let mut samples = self.samples.borrow().clone();
            samples.retain(|x| fragments.iter().all(|frag| x.uri().contains(frag)));

            self.samples_listview_model.extend_from_slice(
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
            self.samples_listview_model.n_items()
        );

        self
    }
}
