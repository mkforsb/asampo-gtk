// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::HashMap;

use anyhow::anyhow;
use gtk::gio::ListStore;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    ext::ClonedHashMapExt,
    view::{dialogs, samples::SampleListEntry, sequences::DrumMachineView},
};

type Result<T> = std::result::Result<T, anyhow::Error>;

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

impl ViewFlags {
    pub fn set_is_sources_add_fs_fields_valid(self, valid: bool) -> ViewFlags {
        ViewFlags {
            sources_add_fs_fields_valid: valid,
            ..self
        }
    }

    pub fn signal_sources_add_fs_begin_browse(self) -> ViewFlags {
        ViewFlags {
            sources_add_fs_begin_browse: true,
            ..self
        }
    }

    pub fn clear_signal_sources_add_fs_begin_browse(self) -> ViewFlags {
        ViewFlags {
            sources_add_fs_begin_browse: false,
            ..self
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
    pub sources_sample_count: HashMap<Uuid, usize>,
    pub samples_list_filter: String,
    pub settings_latency_approx_label: String,
    pub samples_listview_model: ListStore,
    pub sets_export_dialog_view: Option<dialogs::ExportDialogView>,
    pub sets_export_target_dir_entry: String,
    pub sets_export_kind: Option<ExportKind>,
    pub drum_machine: Option<DrumMachineView>,
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
            drum_machine: None,
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

    pub fn init_source_sample_count(self, source_uuid: Uuid) -> Result<ViewValues> {
        if self.sources_sample_count.contains_key(&source_uuid) {
            Err(anyhow!("Failed to init source sample count: UUID in use"))
        } else {
            Ok(ViewValues {
                sources_sample_count: self.sources_sample_count.clone_and_insert(source_uuid, 0),
                ..self
            })
        }
    }

    pub fn source_sample_count_add(self, source_uuid: Uuid, add: usize) -> Result<ViewValues> {
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

    pub fn reset_source_sample_count(self, source_uuid: Uuid) -> Result<ViewValues> {
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

    pub fn clear_sources_add_fs_fields(self) -> ViewValues {
        ViewValues {
            sources_add_fs_name_entry: String::from(""),
            sources_add_fs_path_entry: String::from(""),
            sources_add_fs_extensions_entry: String::from(""),
            ..self
        }
    }

    pub fn set_sources_add_fs_name_entry(self, text: impl Into<String>) -> ViewValues {
        ViewValues {
            sources_add_fs_name_entry: text.into(),
            ..self
        }
    }

    pub fn set_sources_add_fs_path_entry(self, text: impl Into<String>) -> ViewValues {
        ViewValues {
            sources_add_fs_path_entry: text.into(),
            ..self
        }
    }

    pub fn set_sources_add_fs_extensions_entry(self, text: impl Into<String>) -> ViewValues {
        ViewValues {
            sources_add_fs_extensions_entry: text.into(),
            ..self
        }
    }
}
