// MIT License
// 
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::cell::RefCell;
use std::env;

use gtk::gio::ApplicationFlags;
use gtk::glib::subclass::object::ObjectImpl;
use gtk::glib::subclass::prelude::*;
use gtk::glib::subclass::types::ObjectSubclass;
use gtk::glib::{clone, Object};
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow};

#[derive(Default, Debug)]
pub struct SampleListEntryState {
    value: RefCell<Option<libasampo::samples::Sample>>,
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

fn build_filter_bar() -> impl IsA<gtk::Widget> {
    let thebox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    thebox.append(
        &gtk::Label::builder()
            .label("Filter:")
            .margin_top(5)
            .margin_bottom(5)
            .margin_start(5)
            .build(),
    );
    thebox.append(
        &gtk::Entry::builder()
            .hexpand(true)
            .margin_top(5)
            .margin_bottom(5)
            .margin_end(5)
            .build(),
    );

    gtk::Frame::builder().child(&thebox).build()
}

fn build_page_samples() -> impl IsA<gtk::Widget> {
    let thebox = gtk::Box::new(gtk::Orientation::Vertical, 5);

    let model = gtk::gio::ListStore::new::<SampleListEntry>();

    let (tx, rx) = std::sync::mpsc::channel();
    let _ = audiothread::spawn("Audio".to_string(), rx, None);

    use libasampo::samples::SampleTrait;
    use libasampo::sources::SourceTrait;

    let asampo_src = libasampo::sources::Source::FilesystemSource(
        libasampo::sources::file_system_source::FilesystemSource::new(
            env::args()
                .nth(1)
                .expect("Source path should have been given on command line"),
            vec![],
        ),
    );

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
        let sample = SampleListEntryState::from_obj(item.and_dynamic_cast_ref::<SampleListEntry>().unwrap()).value.borrow();
        let uri = sample.as_ref().unwrap().uri();
        tx.send(audiothread::Message::PlaySymphoniaSource(audiothread::SymphoniaSource::from_file(uri).unwrap())).unwrap();
    });

    let selectmodel = gtk::SingleSelection::new(Some(model));

    let listview = gtk::ListView::new(Some(selectmodel), Some(factory));

    listview.set_vexpand(true);
    listview.set_single_click_activate(true);

    listview.settings().set_property("gtk-double-click-time", 0);
    listview.connect_activate(sample_clicked);

    thebox.append(&build_filter_bar());
    thebox.append(&gtk::ScrolledWindow::builder().child(&listview).build());
    thebox
}

fn main() -> glib::ExitCode {
    env_logger::init();

    let app = Application::builder()
        .application_id("se.neode.Asampo")
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_command_line(clone!(@strong app =>  move |_, _| {
        app.activate();
        0
    }));

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(1200)
            .default_height(700)
            .title("Asampo")
            .build();

        let page_settings = gtk::Label::builder().label("Settings").build();
        let page_sources = gtk::Label::builder().label("Sources").build();
        let page_samples = build_page_samples();
        let page_sets = gtk::Label::builder().label("Sets").build();
        let page_sequences = gtk::Label::builder().label("Sequences").build();

        let notebook = gtk::Notebook::new();
        notebook.append_page(
            &page_settings,
            Some(&gtk::Label::builder().label("Settings").build()),
        );
        notebook.append_page(
            &page_sources,
            Some(&gtk::Label::builder().label("Sources").build()),
        );
        notebook.append_page(
            &page_samples,
            Some(&gtk::Label::builder().label("Samples").build()),
        );
        notebook.append_page(
            &page_sets,
            Some(&gtk::Label::builder().label("Sets").build()),
        );
        notebook.append_page(
            &page_sequences,
            Some(&gtk::Label::builder().label("Sequences").build()),
        );

        window.set_child(Some(&notebook));
        window.present();
    });

    app.run()
}
