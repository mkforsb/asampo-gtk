// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

mod view;

use std::cell::RefCell;
use std::env;
// use std::sync::{Arc, RwLock};

use gtk::gio::ApplicationFlags;
use gtk::glib::subclass::object::ObjectImpl;
use gtk::glib::subclass::types::{ObjectSubclass, ObjectSubclassExt};
use gtk::glib::{clone, Object};
use gtk::prelude::*;
use gtk::{glib, Application};
use libasampo::prelude::*;
use libasampo::samples::Sample;
use libasampo::sources::file_system_source::FilesystemSource;
use libasampo::sources::Source;
use view::{AsampoView, AsampoViewState};

#[derive(Default, Debug)]
pub struct SampleListEntryState {
    value: RefCell<Option<Sample>>,
}

#[glib::object_subclass]
impl ObjectSubclass for SampleListEntryState {
    const NAME: &'static str = "SampleListEntry";
    type Type = SampleListEntry;
}

impl ObjectImpl for SampleListEntryState {}

glib::wrapper! {
    pub struct SampleListEntry(ObjectSubclass<SampleListEntryState>);
}

impl SampleListEntry {
    pub fn new(value: libasampo::samples::Sample) -> Self {
        let obj = Object::builder().build();
        let x = SampleListEntryState::from_obj(&obj);
        x.value.replace(Some(value));
        obj
    }
}

// struct AppState {
//     _sources: Arc<RwLock<Vec<Source>>>,
// }

fn main() -> glib::ExitCode {
    env_logger::init();

    gtk::gio::resources_register_include!("resources.gresource")
        .expect("Should be able to register compiled resources.");

    // let _appstate = AppState {
    //     _sources: Arc::new(RwLock::new(Vec::new())),
    // };

    let app = Application::builder()
        .application_id("se.neode.Asampo")
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_command_line(clone!(@strong app =>  move |_, _| {
        app.activate();
        0
    }));

    app.connect_activate(|app| {
        let window = AsampoView::new(app);
        window.present();

        let x = AsampoViewState::from_obj(&window);

        let model = gtk::gio::ListStore::new::<SampleListEntry>();

        let (tx, rx) = std::sync::mpsc::channel();
        let _ = audiothread::spawn("Audio".to_string(), rx, None);

        let asampo_src = Source::FilesystemSource(FilesystemSource::new(
            env::args()
                .nth(1)
                .expect("Source path should have been given on command line"),
            vec![],
        ));

        for x in asampo_src.list().unwrap() {
            model.append(&SampleListEntry::new(x));
        }

        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(None);
            label.set_xalign(0.0);

            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("ListItem")
                .set_child(Some(&label));
        });

        factory.connect_bind(move |_, list_item| {
            let entry = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("ListItem")
                .item()
                .and_downcast::<SampleListEntry>()
                .expect("Entry");
            let label = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("ListItem")
                .child()
                .and_downcast::<gtk::Label>()
                .expect("Label");

            label.set_label(
                SampleListEntryState::from_obj(&entry)
                    .value
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .uri(),
            );
        });

        let sample_clicked = clone!(@strong model, @strong tx => move |_: &_, x| {
            let item = model.item(x);
            let sample =
                SampleListEntryState::from_obj(item.and_dynamic_cast_ref::<SampleListEntry>().unwrap())
                    .value
                    .borrow();
            let uri = sample.as_ref().unwrap().uri();

            tx.send(audiothread::Message::PlaySymphoniaSource(
                audiothread::SymphoniaSource::from_file(uri).unwrap(),
            ))
            .unwrap();
        });

        let selectmodel = gtk::SingleSelection::new(Some(model));

        x.samples_listview.set_vexpand(true);
        x.samples_listview.set_single_click_activate(true);
        x.samples_listview.settings().set_property("gtk-double-click-time", 0);
        x.samples_listview.connect_activate(sample_clicked);

        x.samples_listview.set_model(Some(&selectmodel));
        x.samples_listview.set_factory(Some(&factory));
    });

    app.run()
}
