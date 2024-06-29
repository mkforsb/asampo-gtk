// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::HashMap;

use gtk::gio::ListStore;
use uuid::Uuid;

use crate::{
    config::AppConfig,
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
