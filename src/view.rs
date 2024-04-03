use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib;

#[derive(gtk::CompositeTemplate, Default)]
#[template(resource = "/asampo.ui")]
pub struct AsampoViewState {
    // #[template_child]
    // pub button: gtk::TemplateChild<gtk::Button>,
}

impl gtk::subclass::widget::WidgetImpl for AsampoViewState { }
impl gtk::subclass::window::WindowImpl for AsampoViewState { }
impl gtk::subclass::application_window::ApplicationWindowImpl for AsampoViewState { }

#[glib::object_subclass]
impl gtk::glib::subclass::types::ObjectSubclass for AsampoViewState {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "AsampoView";
    type Type = AsampoView;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &gtk::glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl glib::subclass::object::ObjectImpl for AsampoViewState {
    fn constructed(&self) {
        // Call "constructed" on parent
        self.parent_constructed();

        // Connect to "clicked" signal of `button`
        // self.button.connect_clicked(move |button| {
            // Set the label to "Hello World!" after the button has been clicked on
            // button.set_label("Hello World!");
        // });
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

