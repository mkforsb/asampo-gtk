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
    EventControllerKey, GestureClick,
};
use libasampo::{prelude::*, samples::Sample, samplesets::SampleSet};
use uuid::Uuid;

use crate::{
    model::AppModel,
    update,
    util::{self, gtk_find_child_by_builder_id, resource_as_string, uuidize_builder_template},
    view::AsampoView,
    AppMessage, AppModelPtr, WithModel,
};

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

        label.set_label(
            SampleListEntryState::from_obj(&entry)
                .value
                .borrow()
                .uri()
                .as_str(),
        );
    });

    let selectmodel = gtk::SingleSelection::new(None::<gtk::gio::ListStore>);

    model_ptr.with_model(|model| {
        selectmodel.set_model(Some(model.samples_listmodel()));
        model
    });

    view.samples_listview.set_model(Some(&selectmodel));
    view.samples_listview.set_factory(Some(&factory));

    view.samples_listview
        .connect_activate(clone!(@strong model_ptr, @strong view => move |_, _| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::SampleListSampleSelected(
                    view.samples_listview.model().unwrap().selection().minimum()
                )
            );
        }));

    let clicked = GestureClick::new();

    clicked.connect_released(
        clone!(@strong model_ptr, @strong view => move |_, _, _, _| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::SampleListSampleSelected(
                    view.samples_listview.model().unwrap().selection().minimum()
                )
            );
        }),
    );

    view.samples_listview.add_controller(clicked);

    let keyed = EventControllerKey::new();

    keyed.connect_key_released(
        clone!(@strong model_ptr, @strong view => move |_, key: gtk::gdk::Key, _, _| {
            if key == gtk::gdk::Key::Return {
                return;
            }

            update(
                model_ptr.clone(),
                &view,
                AppMessage::SampleListSampleSelected(
                    view.samples_listview.model().unwrap().selection().minimum()
                )
            );
        }),
    );

    view.samples_listview.add_controller(keyed);

    view.samples_list_filter_entry.connect_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::Entry| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::SamplesFilterChanged(e.text().to_string())
            );
        }),
    );

    view.samples_sidebar_add_to_set_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::SampleSidebarAddToSetClicked);
        }),
    );

    view.samples_sidebar_add_to_prev_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::SampleSidebarAddToMostRecentlyUsedSetClicked
            );
        }),
    );

    for i in 0..16 {
        let popover =
            gtk_find_child_by_builder_id::<gtk::Popover>(view, "-assign-popover").unwrap();

        gtk_find_child_by_builder_id::<gtk::Button>(
            view,
            &format!("samples-sidebar-assign-to-pad-button-{i}"),
        )
        .unwrap()
        .connect_clicked(
            clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
                update(model_ptr.clone(), &view, AppMessage::AssignSampleToPadClicked(i));
                popover.popdown();
            }),
        );
    }
}

pub fn update_samples_sidebar(model_ptr: AppModelPtr, model: AppModel, view: &AsampoView) {
    match model.selected_sample() {
        Some(sample) => {
            view.samples_sidebar_name_label.set_text(sample.name());

            view.samples_sidebar_rate_label
                .set_text(&format!("{} Hz", sample.metadata().rate));

            view.samples_sidebar_format_label
                .set_text(&sample.metadata().src_fmt_display);

            view.samples_sidebar_size_label
                .set_text(&util::readable_size(sample.metadata().size_bytes));

            view.samples_sidebar_length_label
                .set_text(&util::readable_length(sample.metadata().length_millis));

            match sample.source_uuid() {
                Some(uuid) => view.samples_sidebar_source_label.set_text(
                    model
                        .source(*uuid)
                        .map_or("???", |src| src.name().unwrap_or("Unnamed")),
                ),
                None => view.samples_sidebar_source_label.set_text("-"),
            };

            view.samples_sidebar_sets_list.remove_all();

            let mut containing_sets = model
                .sets_map()
                .iter()
                .filter_map(|(uuid, set)| {
                    if set.contains(sample) {
                        Some((uuid, set))
                    } else {
                        None
                    }
                })
                .collect::<Vec<(&Uuid, &SampleSet)>>();

            containing_sets.sort_by(|a, b| a.1.name().cmp(b.1.name()));

            for (uuid, set) in containing_sets {
                let stuff = gtk::Builder::from_string(&uuidize_builder_template(
                    &resource_as_string("/samples-sidebar-sets-member-entry.ui").unwrap(),
                    *uuid,
                ));

                stuff
                    .object::<gtk::Button>(format!("{uuid}-combo-button-label"))
                    .unwrap()
                    .set_label(set.name());

                stuff
                    .object::<gtk::Button>(format!("{uuid}-combo-button-ear"))
                    .unwrap()
                    .connect_clicked(clone!(
                        @strong model_ptr,
                        @strong view,
                        @strong sample,
                        @strong uuid => move |_| {
                            update(
                                model_ptr.clone(),
                                &view,
                                AppMessage::DeleteSampleFromSetClicked(sample.clone(), uuid)
                            );
                        }
                    ));

                view.samples_sidebar_sets_list.append(
                    &stuff
                        .object::<gtk::FlowBoxChild>(format!("{uuid}-entry"))
                        .unwrap(),
                );
            }
        }

        None => {
            view.samples_sidebar_name_label.set_text("-");
            view.samples_sidebar_rate_label.set_text("-");
            view.samples_sidebar_format_label.set_text("-");
            view.samples_sidebar_size_label.set_text("-");
            view.samples_sidebar_length_label.set_text("-");
            view.samples_sidebar_source_label.set_text("-");
        }
    }
}
