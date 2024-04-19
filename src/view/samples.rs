// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::cell::RefCell;

use gtk::{
    glib::{
        self, clone,
        object::Cast,
        subclass::{
            object::ObjectImpl,
            types::{ObjectSubclass, ObjectSubclassExt},
        },
        Object,
    },
    prelude::*,
};
use libasampo::{prelude::*, samples::Sample};

use crate::{update, view::AsampoView, AppMessage, AppModelPtr, WithModel};

#[derive(Default, Debug)]
pub struct SampleListEntryState {
    pub value: RefCell<Sample>,
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
        x.value.replace(value);
        obj
    }
}

impl std::ops::Deref for SampleListEntry {
    type Target = SampleListEntryState;

    fn deref(&self) -> &Self::Target {
        SampleListEntryState::from_obj(self)
    }
}

pub fn setup_samples_page(model_ptr: AppModelPtr, view: &AsampoView) {
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

        label.set_label(SampleListEntryState::from_obj(&entry).value.borrow().uri());
    });

    let sample_clicked = clone!(@strong model_ptr, @strong view => move |_: &gtk::ListView, pos| {
        update(model_ptr.clone(), &view, AppMessage::SampleClicked(pos));
    });

    let selectmodel = gtk::SingleSelection::new(None::<gtk::gio::ListStore>);

    model_ptr.with_model(|model| {
        selectmodel.set_model(Some(&model.viewvalues.samples_listview_model.clone()));
        model
    });

    view.samples_listview
        .settings()
        .set_property("gtk-double-click-time", 0);

    view.samples_listview.connect_activate(sample_clicked);
    view.samples_listview.set_model(Some(&selectmodel));
    view.samples_listview.set_factory(Some(&factory));

    view.samples_list_filter_entry.connect_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::Entry| {
            update(model_ptr.clone(), &view, AppMessage::SamplesFilterChanged(e.text().to_string()));
        }),
    );
}
