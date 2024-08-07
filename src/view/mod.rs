// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

pub mod dialogs;
pub mod menus;
pub mod samples;
pub mod sequences;
pub mod sets;
pub mod settings;
pub mod sources;

use std::ops::Deref;

use gtk::glib;
use gtk::glib::subclass::InitializingObject;
use gtk::subclass::prelude::*;

#[derive(gtk::CompositeTemplate, Default, Debug)]
#[template(resource = "/asampo.ui")]
pub struct AsampoViewState {
    #[template_child(id = "titlebar-stop-button")]
    pub titlebar_stop_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "main-menu-button")]
    pub main_menu_button: gtk::TemplateChild<gtk::MenuButton>,

    #[template_child(id = "progress-popup")]
    pub progress_popup: gtk::TemplateChild<gtk::Frame>,

    #[template_child(id = "progress-popup-progress-bar")]
    pub progress_popup_progress_bar: gtk::TemplateChild<gtk::ProgressBar>,

    #[template_child(id = "stack")]
    pub stack: gtk::TemplateChild<gtk::Stack>,

    #[template_child(id = "settings-output-sample-rate-entry")]
    pub settings_output_sample_rate_entry: gtk::TemplateChild<gtk::DropDown>,

    #[template_child(id = "settings-buffer-size-entry")]
    pub settings_buffer_size_entry: gtk::TemplateChild<gtk::SpinButton>,

    #[template_child(id = "settings-latency-approx-label")]
    pub settings_latency_approx_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "settings-sample-rate-conversion-quality-entry")]
    pub settings_sample_rate_conversion_quality_entry: gtk::TemplateChild<gtk::DropDown>,

    #[template_child(id = "settings-sample-playback-behavior-entry")]
    pub settings_sample_playback_behavior_entry: gtk::TemplateChild<gtk::DropDown>,

    #[template_child(id = "settings-save-workspace-behavior-entry")]
    pub settings_save_workspace_behavior_entry: gtk::TemplateChild<gtk::DropDown>,

    #[template_child(id = "settings-save-changed-sequence-behavior-entry")]
    pub settings_save_changed_sequence_behavior_entry: gtk::TemplateChild<gtk::DropDown>,

    #[template_child(id = "settings-save-changed-set-behavior-entry")]
    pub settings_save_changed_set_behavior_entry: gtk::TemplateChild<gtk::DropDown>,

    #[template_child(id = "settings-synchronize-changed-set-behavior-entry")]
    pub settings_synchronize_changed_set_behavior_entry: gtk::TemplateChild<gtk::DropDown>,

    #[template_child(id = "settings-config-save-path-entry")]
    pub settings_config_save_path_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child(id = "sources-add-frame")]
    pub sources_add_frame: gtk::TemplateChild<gtk::Frame>,

    #[template_child(id = "sources-add-fs-name-entry")]
    pub sources_add_fs_name_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child(id = "sources-add-fs-path-entry")]
    pub sources_add_fs_path_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child(id = "sources-add-fs-path-browse-button")]
    pub sources_add_fs_path_browse_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sources-add-fs-extensions-entry")]
    pub sources_add_fs_extensions_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child(id = "sources-add-fs-add-button")]
    pub sources_add_fs_add_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sources-edit-frame")]
    pub sources_edit_frame: gtk::TemplateChild<gtk::Frame>,

    #[template_child(id = "sources-edit-fs-name-entry")]
    pub sources_edit_fs_name_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child(id = "sources-edit-fs-path-entry")]
    pub sources_edit_fs_path_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child(id = "sources-edit-fs-path-browse-button")]
    pub sources_edit_fs_path_browse_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sources-edit-fs-extensions-entry")]
    pub sources_edit_fs_extensions_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child(id = "sources-edit-fs-save-button")]
    pub sources_edit_fs_save_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sources-edit-fs-cancel-button")]
    pub sources_edit_fs_cancel_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sources-list")]
    pub sources_list: gtk::TemplateChild<gtk::ListBox>,

    #[template_child(id = "samples-list-filter-entry")]
    pub samples_list_filter_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child(id = "samples-listview")]
    pub samples_listview: gtk::TemplateChild<gtk::ListView>,

    #[template_child(id = "samples-sidebar-waveform")]
    pub samples_sidebar_waveform: gtk::TemplateChild<gtk::DrawingArea>,

    #[template_child(id = "samples-sidebar-name-label")]
    pub samples_sidebar_name_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "samples-sidebar-format-label")]
    pub samples_sidebar_format_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "samples-sidebar-rate-label")]
    pub samples_sidebar_rate_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "samples-sidebar-size-label")]
    pub samples_sidebar_size_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "samples-sidebar-length-label")]
    pub samples_sidebar_length_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "samples-sidebar-source-label")]
    pub samples_sidebar_source_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "samples-sidebar-sets-list")]
    pub samples_sidebar_sets_list: gtk::TemplateChild<gtk::FlowBox>,

    #[template_child(id = "samples-sidebar-add-to-set-button")]
    pub samples_sidebar_add_to_set_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "samples-sidebar-add-to-prev-button")]
    pub samples_sidebar_add_to_prev_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sets-list-frame")]
    pub sets_list_frame: gtk::TemplateChild<gtk::Frame>,

    #[template_child(id = "sets-list")]
    pub sets_list: gtk::TemplateChild<gtk::ListBox>,

    #[template_child(id = "sets-add-set-button")]
    pub sets_add_set_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sets-details-name-label")]
    pub sets_details_name_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "sets-details-sample-list-frame")]
    pub sets_details_sample_list_frame: gtk::TemplateChild<gtk::Frame>,

    #[template_child(id = "sets-details-sample-list")]
    pub sets_details_sample_list: gtk::TemplateChild<gtk::ListBox>,

    #[template_child(id = "sets-details-load-drum-machine-button")]
    pub sets_details_load_drum_machine_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sets-details-export-button")]
    pub sets_details_export_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sequences-list-frame")]
    pub sequences_list_frame: gtk::TemplateChild<gtk::Frame>,

    #[template_child(id = "sequences-list")]
    pub sequences_list: gtk::TemplateChild<gtk::ListBox>,

    #[template_child(id = "sequences-add-sequence-button")]
    pub sequences_add_sequence_button: gtk::TemplateChild<gtk::Button>,

    #[template_child(id = "sequences-editor-name-label")]
    pub sequences_editor_name_label: gtk::TemplateChild<gtk::Label>,

    #[template_child(id = "sequences-editor-drum-machine-frame")]
    pub sequences_editor_drum_machine_frame: gtk::TemplateChild<gtk::Frame>,
}

impl WidgetImpl for AsampoViewState {}
impl WindowImpl for AsampoViewState {}
impl ApplicationWindowImpl for AsampoViewState {}

#[glib::object_subclass]
impl ObjectSubclass for AsampoViewState {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "AsampoView";
    type Type = AsampoView;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for AsampoViewState {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

use glib::Object;
use gtk::{gio, Application};

glib::wrapper! {
    pub struct AsampoView(ObjectSubclass<AsampoViewState>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AsampoView {
    pub fn new(app: &Application) -> Self {
        Object::builder().property("application", app).build()
    }
}

impl Deref for AsampoView {
    type Target = AsampoViewState;

    fn deref(&self) -> &Self::Target {
        AsampoViewState::from_obj(self)
    }
}
