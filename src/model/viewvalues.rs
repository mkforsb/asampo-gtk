// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::HashMap;

use anyhow::anyhow;
use gtk::{
    gio::ListStore,
    prelude::{Cast, ListModelExt},
};
use libasampo::samples::{Sample, SampleOps};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    ext::ClonedHashMapExt,
    model::AnyhowResult,
    view::{
        dialogs::{self, ExportDialogView},
        samples::SampleListEntry,
        sequences::DrumMachineView,
        sets::MemberListEntry,
    },
};

#[derive(Debug, Clone)]
pub enum ExportKind {
    PlainCopy,
    Conversion,
}

#[derive(Debug, Clone)]
pub struct ViewValues {
    sources_add_fs_name_entry: String,
    sources_add_fs_path_entry: String,
    sources_add_fs_extensions_entry: String,
    sources_sample_count: HashMap<Uuid, usize>,
    samples_list_filter: String,
    settings_latency_approx_label: String,
    samples_listview_model: ListStore,
    sets_set_pending_deletion: Option<Uuid>,
    sets_export_dialog_view: Option<dialogs::ExportDialogView>,
    sets_export_target_dir_entry: String,
    sets_export_kind: ExportKind,
    sets_export_progress: Option<(usize, usize)>,
    sets_members_listview_model: ListStore,
    drum_machine: Option<DrumMachineView>,
    sequences_seq_pending_deletion: Option<Uuid>,
    savefile_pending_load: Option<String>,
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
            sets_set_pending_deletion: None,
            sets_export_dialog_view: None,
            sets_export_target_dir_entry: String::default(),
            sets_export_kind: ExportKind::PlainCopy,
            sets_export_progress: None,
            sets_members_listview_model: ListStore::new::<MemberListEntry>(),
            drum_machine: None,
            sequences_seq_pending_deletion: None,
            savefile_pending_load: None,
        }
    }
}

impl ViewValues {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            settings_latency_approx_label: config.fmt_latency_approx(),
            ..Self::default()
        }
    }

    pub fn set_latency_approx_label(self, text: String) -> ViewValues {
        ViewValues {
            settings_latency_approx_label: text,
            ..self
        }
    }

    pub fn set_latency_approx_label_by_config(self, config: &AppConfig) -> ViewValues {
        self.set_latency_approx_label(format!(
            "~{:.1} ms",
            config.buffer_size_frames as f32 / config.output_samplerate_hz as f32 * 1000.0
        ))
    }

    pub fn latency_approx_label(&self) -> &String {
        &self.settings_latency_approx_label
    }

    pub fn init_source_sample_count(self, source_uuid: Uuid) -> AnyhowResult<ViewValues> {
        if self.sources_sample_count.contains_key(&source_uuid) {
            Err(anyhow!("Failed to init source sample count: UUID in use"))
        } else {
            Ok(ViewValues {
                sources_sample_count: self.sources_sample_count.clone_and_insert(source_uuid, 0),
                ..self
            })
        }
    }

    pub fn sources_sample_count(&self) -> &HashMap<Uuid, usize> {
        &self.sources_sample_count
    }

    pub fn source_sample_count_add(
        self,
        source_uuid: Uuid,
        add: usize,
    ) -> AnyhowResult<ViewValues> {
        Ok(ViewValues {
            sources_sample_count: self.sources_sample_count.cloned_update_with(|mut m| {
                *(m.get_mut(&source_uuid).ok_or(anyhow!(
                    "Failed to update source sample count: UUID not present"
                )))? += add;
                Ok(m)
            })?,
            ..self
        })
    }

    pub fn reset_source_sample_count(self, source_uuid: Uuid) -> AnyhowResult<ViewValues> {
        if self.sources_sample_count.contains_key(&source_uuid) {
            Ok(ViewValues {
                sources_sample_count: self.sources_sample_count.cloned_update_with(|mut m| {
                    *(m.get_mut(&source_uuid).unwrap()) = 0;
                    Ok(m)
                })?,
                ..self
            })
        } else {
            Err(anyhow!(
                "Failed to reset source sample count: UUID not present"
            ))
        }
    }

    pub fn remove_source_sample_count(self, source_uuid: Uuid) -> AnyhowResult<ViewValues> {
        Ok(ViewValues {
            sources_sample_count: self
                .sources_sample_count
                .clone_and_remove(&source_uuid)
                .map_err(|_| anyhow!("Failed to remove source sample count: UUID not present"))?,
            ..self
        })
    }

    pub fn clear_sources_sample_counts(self) -> ViewValues {
        ViewValues {
            sources_sample_count: HashMap::new(),
            ..self
        }
    }

    pub fn set_add_fs_source_name(self, text: impl Into<String>) -> ViewValues {
        ViewValues {
            sources_add_fs_name_entry: text.into(),
            ..self
        }
    }

    pub fn set_add_fs_source_name_if_empty(self, text: impl Into<String>) -> ViewValues {
        if self.sources_add_fs_name_entry.is_empty() {
            ViewValues {
                sources_add_fs_name_entry: text.into(),
                ..self
            }
        } else {
            self
        }
    }

    pub fn add_fs_source_name(&self) -> &String {
        &self.sources_add_fs_name_entry
    }

    pub fn set_add_fs_source_path(self, text: impl Into<String>) -> ViewValues {
        ViewValues {
            sources_add_fs_path_entry: text.into(),
            ..self
        }
    }

    pub fn add_fs_source_path(&self) -> &String {
        &self.sources_add_fs_path_entry
    }

    pub fn set_add_fs_source_extensions(self, text: impl Into<String>) -> ViewValues {
        ViewValues {
            sources_add_fs_extensions_entry: text.into(),
            ..self
        }
    }

    pub fn add_fs_source_extensions(&self) -> &String {
        &self.sources_add_fs_extensions_entry
    }

    pub fn clear_add_fs_source_fields(self) -> ViewValues {
        ViewValues {
            sources_add_fs_name_entry: String::from(""),
            sources_add_fs_path_entry: String::from(""),
            sources_add_fs_extensions_entry: String::from(""),
            ..self
        }
    }

    pub fn get_listed_sample(&self, index: u32) -> AnyhowResult<Sample> {
        // TODO: is it possible to avoid cloning here?
        Ok(self
            .samples_listview_model
            .item(index)
            .ok_or(anyhow!("Failed to fetch sample: index not populated"))?
            .dynamic_cast_ref::<SampleListEntry>()
            .ok_or(anyhow!("Failed to fetch sample: GLib type cast failure"))?
            .value
            .borrow()
            .clone())
    }

    pub fn set_samples_list_filter(self, text: impl Into<String>) -> ViewValues {
        ViewValues {
            samples_list_filter: text.into(),
            ..self
        }
    }

    pub fn set_set_pending_deletion(self, maybe_uuid: Option<Uuid>) -> ViewValues {
        ViewValues {
            sets_set_pending_deletion: maybe_uuid,
            ..self
        }
    }

    pub fn set_pending_deletion(&self) -> Option<Uuid> {
        self.sets_set_pending_deletion
    }

    pub fn set_export_dialog_view(self, maybe_view: Option<ExportDialogView>) -> ViewValues {
        ViewValues {
            sets_export_dialog_view: maybe_view,
            ..self
        }
    }

    pub fn export_dialog_view(&self) -> Option<&ExportDialogView> {
        self.sets_export_dialog_view.as_ref()
    }

    pub fn set_export_target_dir(self, text: impl Into<String>) -> ViewValues {
        ViewValues {
            sets_export_target_dir_entry: text.into(),
            ..self
        }
    }

    pub fn export_target_dir(&self) -> &String {
        &self.sets_export_target_dir_entry
    }

    pub fn set_export_kind(self, kind: ExportKind) -> ViewValues {
        ViewValues {
            sets_export_kind: kind,
            ..self
        }
    }

    pub fn export_kind(&self) -> &ExportKind {
        &self.sets_export_kind
    }

    pub fn init_export_progress(self, total_items: usize) -> ViewValues {
        ViewValues {
            sets_export_progress: Some((0, total_items)),
            ..self
        }
    }

    pub fn export_progress(&self) -> Option<(usize, usize)> {
        self.sets_export_progress
    }

    pub fn set_export_items_completed(self, completed: usize) -> AnyhowResult<ViewValues> {
        let sets_export_progress = Some(
            self.sets_export_progress
                .map(|(_prev_completed, total)| (completed, total))
                .ok_or(anyhow!("Export progress not initialized"))?,
        );

        Ok(ViewValues {
            sets_export_progress,
            ..self
        })
    }

    pub fn reset_export_progress(self) -> ViewValues {
        ViewValues {
            sets_export_progress: None,
            ..self
        }
    }

    pub fn populate_samples_listmodel(&self, samples: &[Sample]) {
        let filter = &self.samples_list_filter;
        self.samples_listview_model.remove_all();

        if filter.is_empty() {
            self.samples_listview_model.extend_from_slice(
                samples
                    .iter()
                    .map(|s| SampleListEntry::new(s.clone()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
        } else {
            let fragments = filter
                .split(' ')
                .map(|s| s.to_string().to_lowercase())
                .collect::<Vec<_>>();

            self.samples_listview_model.extend_from_slice(
                samples
                    .iter()
                    .filter_map(|sample| {
                        if fragments
                            .iter()
                            .all(|frag| sample.uri().as_str().to_lowercase().contains(frag))
                        {
                            Some(SampleListEntry::new(sample.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
            )
        }

        log::log!(
            log::Level::Debug,
            "Showing {} samples",
            self.samples_listview_model.n_items()
        );
    }

    pub fn samples_listmodel(&self) -> &gtk::gio::ListStore {
        &self.samples_listview_model
    }

    pub fn sets_members_listmodel(&self) -> &gtk::gio::ListStore {
        &self.sets_members_listview_model
    }

    pub fn set_drum_machine_view(self, maybe_view: Option<DrumMachineView>) -> ViewValues {
        ViewValues {
            drum_machine: maybe_view,
            ..self
        }
    }

    pub fn drum_machine_view(&self) -> Option<&DrumMachineView> {
        self.drum_machine.as_ref()
    }

    pub fn set_sequence_pending_deletion(self, maybe_uuid: Option<Uuid>) -> ViewValues {
        ViewValues {
            sequences_seq_pending_deletion: maybe_uuid,
            ..self
        }
    }

    pub fn sequence_pending_deletion(&self) -> Option<Uuid> {
        self.sequences_seq_pending_deletion
    }

    pub fn set_savefile_pending_load(self, maybe_filename: Option<String>) -> ViewValues {
        ViewValues {
            savefile_pending_load: maybe_filename,
            ..self
        }
    }

    pub fn savefile_pending_load(&self) -> Option<&String> {
        self.savefile_pending_load.as_ref()
    }
}
