// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
};

use anyhow::anyhow;
use gtk::{gio::ListStore, prelude::*};
use libasampo::{
    prelude::*,
    samples::Sample,
    samplesets::{export::ExportJobMessage, SampleSet},
    sequences::drumkit_render_thread,
    sources::Source,
};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    ext::{ClonedHashMapExt, ClonedVecExt},
    view::{dialogs, samples::SampleListEntry},
};

#[derive(Debug, Clone)]
pub struct ViewFlags {
    pub view_sensitive: bool,
    pub sources_add_fs_fields_valid: bool,
    pub sources_add_fs_begin_browse: bool,
    pub samples_sidebar_add_to_set_show_dialog: bool,
    pub samples_sidebar_add_to_prev_enabled: bool,
    pub sets_add_set_show_dialog: bool,
    pub sets_export_enabled: bool,
    pub sets_export_show_dialog: bool,
    pub sets_export_begin_browse: bool,
    pub sets_export_fields_valid: bool,
}

impl Default for ViewFlags {
    fn default() -> Self {
        ViewFlags {
            view_sensitive: true,
            sources_add_fs_fields_valid: false,
            sources_add_fs_begin_browse: false,
            samples_sidebar_add_to_set_show_dialog: false,
            samples_sidebar_add_to_prev_enabled: false,
            sets_add_set_show_dialog: false,
            sets_export_enabled: false,
            sets_export_show_dialog: false,
            sets_export_begin_browse: false,
            sets_export_fields_valid: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportState {
    Exporting,
    Finished,
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
    pub sources_sample_count: HashMap<Uuid, usize>,
    pub samples_list_filter: String,
    pub settings_latency_approx_label: String,
    pub samples_listview_model: ListStore,
    pub sets_export_dialog_view: Option<dialogs::ExportDialogView>,
    pub sets_export_target_dir_entry: String,
    pub sets_export_kind: Option<ExportKind>,
}

impl Default for ViewValues {
    fn default() -> Self {
        ViewValues {
            sources_add_fs_name_entry: String::default(),
            sources_add_fs_path_entry: String::default(),
            sources_add_fs_extensions_entry: String::default(),
            sources_sample_count: HashMap::new(),
            samples_list_filter: String::default(),
            settings_latency_approx_label: String::default(),
            samples_listview_model: ListStore::new::<SampleListEntry>(),
            sets_export_dialog_view: None,
            sets_export_target_dir_entry: String::default(),
            sets_export_kind: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppModel {
    pub config: Option<AppConfig>,
    pub config_save_timeout: Option<std::time::Instant>,
    pub savefile: Option<String>,
    pub viewflags: ViewFlags,
    pub viewvalues: ViewValues,
    pub audiothread_tx: Option<Sender<audiothread::Message>>,
    pub _audiothread_handle: Option<Rc<JoinHandle<()>>>,
    pub dks_render_thread_tx: Option<Sender<drumkit_render_thread::Message>>,
    pub sources: HashMap<Uuid, Source>,
    pub sources_order: Vec<Uuid>,
    pub sources_loading: HashMap<Uuid, Rc<Receiver<Result<Sample, libasampo::errors::Error>>>>,
    pub samples: Rc<RefCell<Vec<Sample>>>,
    pub samplelist_selected_sample: Option<Sample>,
    pub sets: HashMap<Uuid, SampleSet>,
    pub sets_order: Vec<Uuid>,
    pub sets_selected_set: Option<Uuid>,
    pub sets_most_recently_used_uuid: Option<Uuid>,
    pub sets_export_state: Option<ExportState>,
    pub sets_export_progress: Option<(usize, usize)>,
    pub export_job_rx: Option<Rc<Receiver<ExportJobMessage>>>,
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

        let dks_render_thread_tx = if let Some(audiothread_tx) = &tx {
            use drumkit_render_thread as dkr;

            let (dks_render_thread_tx, dks_render_thread_rx) = mpsc::channel::<dkr::Message>();
            let _ = dkr::spawn(audiothread_tx.clone(), dks_render_thread_rx);

            Some(dks_render_thread_tx)
        } else {
            None
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
            dks_render_thread_tx,
            sources: HashMap::new(),
            sources_order: Vec::new(),
            sources_loading: HashMap::new(),
            samples: Rc::new(RefCell::new(Vec::new())),
            samplelist_selected_sample: None,
            sets: HashMap::new(),
            sets_order: Vec::new(),
            sets_selected_set: None,
            sets_most_recently_used_uuid: None,
            sets_export_state: None,
            sets_export_progress: None,
            export_job_rx: None,
        }
    }

    pub fn add_source(self, source: Source) -> Self {
        AppModel {
            viewvalues: ViewValues {
                sources_sample_count: self
                    .viewvalues
                    .sources_sample_count
                    .clone_and_insert(*source.uuid(), 0),
                ..self.viewvalues
            },
            sources_order: self.sources_order.clone_and_push(*source.uuid()),
            sources: self.sources.clone_and_insert(*source.uuid(), source),
            ..self
        }
    }

    pub fn enable_source(self, uuid: &Uuid) -> Result<Self, anyhow::Error> {
        Ok(AppModel {
            viewvalues: ViewValues {
                sources_sample_count: self.viewvalues.sources_sample_count.cloned_update_with(
                    |mut m| {
                        *(m.get_mut(uuid).unwrap()) = 0;
                        Ok(m)
                    },
                )?,
                ..self.viewvalues
            },
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
        let model = self.disable_source(uuid)?;

        Ok(AppModel {
            viewvalues: ViewValues {
                sources_sample_count: model
                    .viewvalues
                    .sources_sample_count
                    .clone_and_remove(uuid)?,
                ..model.viewvalues
            },
            sources_order: model.sources_order.clone_and_remove(uuid)?,
            sources: model.sources.clone_and_remove(uuid)?,
            ..model
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
            "Showing {} samples",
            self.viewvalues.samples_listview_model.n_items()
        );
    }

    pub fn add_sampleset(self, set: SampleSet) -> Self {
        AppModel {
            sets_order: self.sets_order.clone_and_push(*set.uuid()),
            sets: self.sets.clone_and_insert(*set.uuid(), set),
            ..self
        }
    }

    #[cfg(test)]
    pub fn remove_sampleset(self, uuid: &Uuid) -> Result<Self, anyhow::Error> {
        Ok(AppModel {
            sets_order: self.sets_order.clone_and_remove(uuid)?,
            sets: self.sets.clone_and_remove(uuid)?,
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

        assert!(model.sets.contains_key(set.uuid()));
        assert_eq!(model.sets.get(set.uuid()).unwrap().name(), "Favorites");

        let model = model.remove_sampleset(set.uuid()).unwrap();

        assert!(!model.sets.contains_key(set.uuid()));
    }
}
