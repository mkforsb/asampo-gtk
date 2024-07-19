// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{glib::clone, prelude::*, EventControllerKey, GestureClick};
use libasampo::{samples::SampleOps, samplesets::SampleSetOps};

use crate::{
    model::{AppModel, AppModelPtr},
    update,
    util::{idize_builder_template, resource_as_string, uuidize_builder_template},
    view::AsampoView,
    AppMessage,
};

pub fn setup_sets_page(model_ptr: AppModelPtr, view: &AsampoView) {
    view.sets_add_set_button
        .connect_clicked(clone!(@strong model_ptr, @strong view => move |_| {
            update(model_ptr.clone(), &view, AppMessage::AddSampleSetClicked);
        }));

    view.sets_details_export_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::SampleSetDetailsExportClicked);
        }),
    );
}

pub fn update_samplesets_list(model_ptr: AppModelPtr, model: AppModel, view: &AsampoView) {
    view.sets_list.remove_all();

    view.sets_list_frame
        .set_label(Some(&format!("Sets ({})", model.sets_map().len())));

    for set in model.sets_list().iter() {
        let uuid = set.uuid();

        let objects = gtk::Builder::from_string(&uuidize_builder_template(
            &resource_as_string("/sets-list-row.ui").unwrap(),
            uuid,
        ));

        let row = objects
            .object::<gtk::ListBoxRow>(format!("{uuid}-row"))
            .unwrap();

        let name_label = objects
            .object::<gtk::Label>(format!("{uuid}-name-label"))
            .unwrap();

        name_label.set_text(model.set(uuid).unwrap().name());

        let clicked = GestureClick::new();

        clicked.connect_pressed(|e: &GestureClick, _, _, _| {
            e.widget().activate();
        });

        row.add_controller(clicked);

        let keyup = EventControllerKey::new();

        keyup.connect_key_released(clone!(@strong model_ptr, @strong view, @strong uuid =>
            move |_: &EventControllerKey, _, _, _| {
                update(model_ptr.clone(), &view, AppMessage::SampleSetSelected(uuid));
            }
        ));

        row.add_controller(keyup);

        row.connect_activate(
            clone!(@strong model_ptr, @strong view, @strong uuid => move |_: &gtk::ListBoxRow| {
                update(model_ptr.clone(), &view, AppMessage::SampleSetSelected(uuid));
            }),
        );

        view.sets_list.append(&row);
    }
}

pub fn update_samplesets_detail(model_ptr: AppModelPtr, model: AppModel, view: &AsampoView) {
    view.sets_details_sample_list.remove_all();

    match model.selected_set().and_then(|uuid| model.set(uuid).ok()) {
        Some(set) => {
            view.sets_details_name_label.set_text(set.name());

            view.sets_details_sample_list_frame
                .set_label(Some(&format!("Samples ({})", set.len())));

            for (row_index, sample) in set.list().iter().enumerate() {
                let objects = gtk::Builder::from_string(&idize_builder_template(
                    &resource_as_string("/sets-details-sample-list-row.ui").unwrap(),
                    row_index,
                ));

                let row = objects
                    .object::<gtk::ListBoxRow>(format!("{row_index}-row"))
                    .unwrap();

                let name_label = objects
                    .object::<gtk::Label>(format!("{row_index}-name-label"))
                    .unwrap();
                name_label.set_label(sample.name());

                let clicked = GestureClick::new();

                clicked.connect_pressed(
                    clone!(@strong model_ptr, @strong view => move |gst: &GestureClick, _, _, _| {
                        gst.widget().activate();

                    }),
                );

                row.add_controller(clicked);

                let bound_sample = (*sample).clone();

                row.connect_activate(
                    clone!(@strong model_ptr, @strong view => move |_: &gtk::ListBoxRow| {
                        update(
                            model_ptr.clone(),
                            &view,
                            AppMessage::SampleSetSampleSelected(bound_sample.clone())
                        );
                    }),
                );

                view.sets_details_sample_list.append(&row);
            }
        }
        None => {
            view.sets_details_name_label.set_text("");
        }
    }
}
