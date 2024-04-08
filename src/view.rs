use std::ops::Deref;

use gtk::glib;
use gtk::glib::subclass::InitializingObject;
use gtk::subclass::prelude::*;

#[derive(gtk::CompositeTemplate, Default, Debug)]
#[template(resource = "/asampo.ui")]
pub struct AsampoViewState {
    #[template_child]
    pub sources_add_frame: gtk::TemplateChild<gtk::Frame>,

    #[template_child]
    pub sources_add_fs_name_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child]
    pub sources_add_fs_path_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child]
    pub sources_add_fs_path_browse_button: gtk::TemplateChild<gtk::Button>,

    #[template_child]
    pub sources_add_fs_extensions_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child]
    pub sources_add_fs_add_button: gtk::TemplateChild<gtk::Button>,

    #[template_child]
    pub sources_edit_frame: gtk::TemplateChild<gtk::Frame>,

    #[template_child]
    pub sources_edit_fs_name_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child]
    pub sources_edit_fs_path_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child]
    pub sources_edit_fs_path_browse_button: gtk::TemplateChild<gtk::Button>,

    #[template_child]
    pub sources_edit_fs_extensions_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child]
    pub sources_edit_fs_save_button: gtk::TemplateChild<gtk::Button>,

    #[template_child]
    pub sources_edit_fs_cancel_button: gtk::TemplateChild<gtk::Button>,

    #[template_child]
    pub sources_list: gtk::TemplateChild<gtk::ListBox>,

    #[template_child]
    pub samples_list_filter_entry: gtk::TemplateChild<gtk::Entry>,

    #[template_child]
    pub samples_listview: gtk::TemplateChild<gtk::ListView>,
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
