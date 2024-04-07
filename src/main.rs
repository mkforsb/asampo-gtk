// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

mod view;

use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use audiothread::Message;
use gtk::gio::ApplicationFlags;
use gtk::glib::subclass::object::ObjectImpl;
use gtk::glib::subclass::types::{ObjectSubclass, ObjectSubclassExt};
use gtk::glib::{clone, Object};
use gtk::{glib, Application};
use gtk::{prelude::*, GestureClick};
use libasampo::prelude::*;
use libasampo::samples::Sample;
use libasampo::sources::file_system_source::FilesystemSource;
use libasampo::sources::Source;
use uuid::Uuid;
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

fn update_sources_list(appstate: Rc<AppState>, view: &AsampoView) {
    view.sources_list.remove_all();

    for uuid in appstate.sources_order.borrow().iter() {
        let objects = gtk::Builder::from_string(indoc::indoc! {r#"
            <interface>
                <object class="GtkListBoxRow">
                    <child>
                        <object class="GtkBox">
                            <property name="orientation">GTK_ORIENTATION_HORIZONTAL</property>
                            <child>
                                <object class="GtkCheckButton">
                                    <property name="margin-top">10</property>
                                    <property name="margin-start">10</property>
                                    <property name="margin-bottom">10</property>
                                    <property name="tooltip-text">Enable?</property>
                                </object>
                            </child>
                            <child>
                                <object class="GtkLabel">
                                    <property name="label"></property>
                                    <property name="halign">GTK_ALIGN_FILL</property>
                                    <property name="hexpand">true</property>
                                    <property name="xalign">0.0</property>
                                    <property name="margin_start">10</property>
                                    <property name="margin_top">10</property>
                                    <property name="margin_bottom">10</property>
                                </object>
                            </child>
                            <child>
                                <object class="GtkButton">
                                    <property name="label">Delete</property>
                                    <property name="margin_end">16</property>
                                </object>
                            </child>
                        </object>
                    </child>
                </object>
            </interface>
        "#})
        .objects();

        let row = objects[0].dynamic_cast_ref::<gtk::ListBoxRow>().unwrap();

        let hbox_raw = row.child().unwrap();
        let hbox = hbox_raw.dynamic_cast_ref::<gtk::Box>().unwrap();

        let checkbutton_raw = hbox.first_child().unwrap();
        let checkbutton = checkbutton_raw
            .dynamic_cast_ref::<gtk::CheckButton>()
            .unwrap();

        if appstate.sources.borrow().get(uuid).unwrap().is_enabled() {
            checkbutton.activate();
        }

        checkbutton.connect_toggled(
            clone!(@strong appstate, @strong uuid, @strong view => move |e: &gtk::CheckButton| {
                appstate.sources.borrow_mut().get_mut(&uuid).unwrap().set_enabled(e.is_active());

                if e.is_active() {
                    enable_source(appstate.clone(), &view, &uuid);
                } else {
                    disable_source(appstate.clone(), &view, &uuid);
                }
            }),
        );

        row.child()
            .unwrap()
            .dynamic_cast_ref::<gtk::Box>()
            .unwrap()
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .dynamic_cast_ref::<gtk::Label>()
            .unwrap()
            .set_label(
                appstate
                    .sources
                    .borrow()
                    .get(uuid)
                    .unwrap()
                    .name()
                    .unwrap_or("Unnamed"),
            );

        let clicked = GestureClick::new();

        clicked.connect_pressed(|e: &GestureClick, _, _, _| {
            e.widget().activate();
        });

        row.add_controller(clicked);

        view.sources_list.append(row);
    }
}

fn enable_source(appstate: Rc<AppState>, view: &AsampoView, source_uuid: &Uuid) {
    {
        let mut samples = appstate.samples.borrow_mut();

        for sample in appstate
            .sources
            .borrow()
            .get(source_uuid)
            .unwrap()
            .list()
            .unwrap()
        {
            samples.push(sample);
        }
    }

    apply_samples_filter(appstate.clone(), &view.samples_filter.text());
}

fn disable_source(appstate: Rc<AppState>, view: &AsampoView, source_uuid: &Uuid) {
    appstate
        .samples
        .borrow_mut()
        .retain(|s| s.source_uuid() != Some(source_uuid));
    apply_samples_filter(appstate.clone(), &view.samples_filter.text());
}

fn setup_sources_page(appstate: Rc<AppState>, view: &AsampoView) {
    let msgbox = clone!(@strong view => move |msg: String| {
        let mbox = gtk::AlertDialog::builder().modal(true).message(msg).buttons(["Ok"]).build();
        mbox.show(Some(&view));
    });

    view.source_add_fs_add_button
        .connect_clicked(clone!(@strong view => move |_| {
            let mut failures: Vec<bool> = Vec::new();

            failures.push(view.source_add_fs_name_entry.text_length() < 1);
            failures.push(view.source_add_fs_path_entry.text_length() < 1);
            failures.push(view.source_add_fs_extensions_entry.text_length() < 1);

            if failures.contains(&true) {
                msgbox(String::from("Fields not filled correctly"));
            } else {
                let new_source = Source::FilesystemSource(FilesystemSource::new_named(
                    view.source_add_fs_name_entry.text().to_string(),
                    view.source_add_fs_path_entry.text().to_string(),
                    view.source_add_fs_extensions_entry
                        .text()
                        .split(',')
                        .map(|x| x.trim().to_string())
                        .collect::<Vec<_>>(),
                ));

                let uuid = *new_source.uuid();

                appstate.sources_order.borrow_mut().push(uuid);
                appstate.sources.borrow_mut().insert(uuid, new_source);

                update_sources_list(appstate.clone(), &view);
                enable_source(appstate.clone(), &view, &uuid);
            }
        }));
}

fn apply_samples_filter(appstate: Rc<AppState>, filter: &str) {
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

fn setup_samples_page(appstate: Rc<AppState>, view: &AsampoView) {
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
            apply_samples_filter(appstate.clone(), entry.text().as_ref());
        }),
    );

    apply_samples_filter(appstate, "");
}

struct AppState {
    sources: Rc<RefCell<HashMap<Uuid, Source>>>,
    sources_order: Rc<RefCell<Vec<Uuid>>>,
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
            sources: Rc::new(RefCell::new(HashMap::new())),
            sources_order: Rc::new(RefCell::new(Vec::new())),
            samples: Rc::new(RefCell::new(Vec::new())),
            samples_listview_model: gtk::gio::ListStore::new::<SampleListEntry>(),
            audiothread_handle: Rc::new(audiothread::spawn(
                rx,
                Some(
                    audiothread::Opts::default()
                        .with_sr_conv_quality(audiothread::Quality::Fastest)
                        .with_bufsize_n_stereo_samples(1024),
                ),
            )),
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

        setup_sources_page(appstate.clone(), &window);
        setup_samples_page(appstate.clone(), &window);

        window.present();
    });

    app.run()
}
