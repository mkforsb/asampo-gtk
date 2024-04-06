// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

mod view;

use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use audiothread::Message;
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
    value: RefCell<Option<Sample>>, // FIXME: the `Option` is there only for its `Default` impl
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
    pub fn new(value: Sample) -> Self {
        let obj = Object::builder().build();
        let x = SampleListEntryState::from_obj(&obj);
        x.value.replace(Some(value));
        obj
    }
}

fn update_samples_list(appstate: Rc<AppState>, filter: &str) {
    appstate.samples_listview_model.remove_all();

    if filter.is_empty() {
        let samples = appstate
            .samples
            .borrow()
            .iter()
            .map(|s| SampleListEntry::new(s.clone()))
            .collect::<Vec<_>>();
        appstate
            .samples_listview_model
            .extend_from_slice(samples.as_slice());
    } else {
        let fragments = filter.split(' ').map(|s| s.to_string()).collect::<Vec<_>>();

        let mut samples = appstate.samples.borrow().clone();
        samples.retain(|x| fragments.iter().all(|frag| x.uri().contains(frag)));

        appstate.samples_listview_model.extend_from_slice(
            samples
                .iter()
                .map(|s| SampleListEntry::new(s.clone()))
                .collect::<Vec<_>>()
                .as_slice(),
        );
    }
}

fn setup_samples_list(appstate: Rc<AppState>, view: &AsampoView) {
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

    let sample_clicked = clone!(@strong appstate.audiothread_tx as tx => move |list_view: &gtk::ListView, pos| {
        let item = list_view.model().expect("").item(pos);
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

    let selectmodel = gtk::SingleSelection::new(Some(appstate.samples_listview_model.clone()));
    let viewstate = AsampoViewState::from_obj(view);

    viewstate
        .samples_listview
        .settings()
        .set_property("gtk-double-click-time", 0);

    viewstate.samples_listview.connect_activate(sample_clicked);
    viewstate.samples_listview.set_model(Some(&selectmodel));
    viewstate.samples_listview.set_factory(Some(&factory));

    viewstate.samples_filter.connect_changed(
        clone!(@strong appstate => move |entry: &gtk::Entry| {
            update_samples_list(appstate.clone(), entry.text().as_ref());
        }),
    );

    update_samples_list(appstate, "");
}

struct AppState {
    sources: Rc<RefCell<Vec<Source>>>,
    samples: Rc<RefCell<Vec<Sample>>>,
    samples_listview_model: gtk::gio::ListStore, // cloning seems to behave like Rc
    audiothread_tx: Sender<Message>,             // can clone to create more senders

    #[allow(unused)]
    audiothread_handle: Rc<JoinHandle<()>>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        AppState {
            sources: Rc::new(RefCell::new(Vec::new())),
            samples: Rc::new(RefCell::new(Vec::new())),
            samples_listview_model: gtk::gio::ListStore::new::<SampleListEntry>(),
            audiothread_handle: Rc::new(audiothread::spawn("Asampo Audio".to_string(), rx, None)),
            audiothread_tx: tx,
        }
    }
}

fn main() -> glib::ExitCode {
    env_logger::init();

    gtk::gio::resources_register_include!("resources.gresource")
        .expect("Should be able to register compiled GTK resources.");

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
        let appstate = Rc::new(AppState::new());

        appstate
            .sources
            .borrow_mut()
            .push(Source::FilesystemSource(FilesystemSource::new(
                env::args()
                    .nth(1)
                    .expect("Source path should have been given on command line"),
                vec![],
            )));

        {
            let mut samples = appstate.samples.borrow_mut();

            for sample in appstate
                .sources
                .borrow_mut()
                .first()
                .unwrap()
                .list()
                .unwrap()
            {
                samples.push(sample);
            }
        }

        setup_samples_list(appstate, &window);

        window.present();
    });

    app.run()
}
