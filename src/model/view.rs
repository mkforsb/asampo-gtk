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
    model::{AppModel, ModelResult},
    view::{dialogs, samples::SampleListEntry, sequences::DrumMachineView},
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
    pub fn new(config: Option<&AppConfig>) -> Self {
        Self {
            settings_latency_approx_label: match &config {
                Some(conf) => conf.fmt_latency_approx(),
                None => "???".to_string(),
            },
            ..Self::default()
        }
    }
}

pub trait ViewModelOps {
    fn set_latency_approx_label(self, text: String) -> AppModel;
    fn set_latency_approx_label_by_config(self, config: &AppConfig) -> AppModel;
    fn init_source_sample_count(self, source_uuid: Uuid) -> ModelResult;
    fn reset_source_sample_count(self, source_uuid: Uuid) -> ModelResult;
    fn set_is_sources_add_fs_fields_valid(self, valid: bool) -> AppModel;
    fn clear_sources_add_fs_fields(self) -> AppModel;
    fn set_sources_add_fs_name_entry(self, text: impl Into<String>) -> AppModel;
    fn set_sources_add_fs_path_entry(self, text: impl Into<String>) -> AppModel;
    fn set_sources_add_fs_extensions_entry(self, text: impl Into<String>) -> AppModel;
    fn signal_sources_add_fs_begin_browse(self) -> AppModel;
    fn clear_signal_sources_add_fs_begin_browse(self) -> AppModel;
}

impl ViewModelOps for AppModel {
    fn set_latency_approx_label(self, text: String) -> AppModel {
        AppModel {
            viewvalues: ViewValues {
                settings_latency_approx_label: text,
                ..self.viewvalues
            },
            ..self
        }
    }

    fn set_latency_approx_label_by_config(self, config: &AppConfig) -> AppModel {
        self.set_latency_approx_label(format!(
            "~{:.1} ms",
            config.buffer_size_frames as f32 / config.output_samplerate_hz as f32 * 1000.0
        ))
    }

    fn init_source_sample_count(self, source_uuid: Uuid) -> ModelResult {
        if self
            .viewvalues
            .sources_sample_count
            .contains_key(&source_uuid)
        {
            Err(anyhow!("Failed to init source sample count: UUID in use"))
        } else {
            Ok(AppModel {
                viewvalues: ViewValues {
                    sources_sample_count: self
                        .viewvalues
                        .sources_sample_count
                        .clone_and_insert(source_uuid, 0),
                    ..self.viewvalues
                },
                ..self
            })
        }
    }

    fn reset_source_sample_count(self, source_uuid: Uuid) -> ModelResult {
        if self
            .viewvalues
            .sources_sample_count
            .contains_key(&source_uuid)
        {
            Ok(AppModel {
                viewvalues: ViewValues {
                    sources_sample_count: self.viewvalues.sources_sample_count.cloned_update_with(
                        |mut m| {
                            *(m.get_mut(&source_uuid).unwrap()) = 0;
                            Ok(m)
                        },
                    )?,
                    ..self.viewvalues
                },
                ..self
            })
        } else {
            Err(anyhow!(
                "Failed to reset source sample count: UUID not present"
            ))
        }
    }

    fn set_is_sources_add_fs_fields_valid(self, valid: bool) -> AppModel {
        AppModel {
            viewflags: ViewFlags {
                sources_add_fs_fields_valid: valid,
                ..self.viewflags
            },
            ..self
        }
    }

    fn clear_sources_add_fs_fields(self) -> AppModel {
        AppModel {
            viewvalues: ViewValues {
                sources_add_fs_name_entry: String::from(""),
                sources_add_fs_path_entry: String::from(""),
                sources_add_fs_extensions_entry: String::from(""),
                ..self.viewvalues
            },
            ..self
        }
    }

    fn set_sources_add_fs_name_entry(self, text: impl Into<String>) -> AppModel {
        AppModel {
            viewvalues: ViewValues {
                sources_add_fs_name_entry: text.into(),
                ..self.viewvalues
            },
            ..self
        }
    }

    fn set_sources_add_fs_path_entry(self, text: impl Into<String>) -> AppModel {
        AppModel {
            viewvalues: ViewValues {
                sources_add_fs_path_entry: text.into(),
                ..self.viewvalues
            },
            ..self
        }
    }

    fn set_sources_add_fs_extensions_entry(self, text: impl Into<String>) -> AppModel {
        AppModel {
            viewvalues: ViewValues {
                sources_add_fs_extensions_entry: text.into(),
                ..self.viewvalues
            },
            ..self
        }
    }

    fn signal_sources_add_fs_begin_browse(self) -> AppModel {
        AppModel {
            viewflags: ViewFlags {
                sources_add_fs_begin_browse: true,
                ..self.viewflags
            },
            ..self
        }
    }

    fn clear_signal_sources_add_fs_begin_browse(self) -> AppModel {
        AppModel {
            viewflags: ViewFlags {
                sources_add_fs_begin_browse: false,
                ..self.viewflags
            },
            ..self
        }
    }
}
