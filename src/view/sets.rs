// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{
    glib::{self, clone},
    prelude::*,
    EventControllerKey, GestureClick,
};
use libasampo::{samples::SampleOps, samplesets::SampleSetOps};

use crate::{
    appmessage::AppMessage,
    ext::OptionMapExt,
    labels::DRUM_LABELS,
    model::{AppModel, AppModelPtr},
    update,
    util::{
        idize_builder_template, resource_as_string, set_dropdown_choice, uuidize_builder_template,
    },
    view::AsampoView,
};

pub fn setup_sets_page(model_ptr: AppModelPtr, view: &AsampoView) {
    view.sets_add_set_button
        .connect_clicked(clone!(@strong model_ptr, @strong view => move |_| {
            update(model_ptr.clone(), &view, AppMessage::AddSampleSetClicked);
        }));

    view.sets_details_load_drum_machine_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::SampleSetDetailsLoadInDrumMachineClicked);
        }),
    );

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

        view.sets_list.append(&row);

        if Some(uuid) == model.selected_set() {
            row.activate();
        }

        row.connect_activate(
            clone!(@strong model_ptr, @strong view, @strong uuid => move |_: &gtk::ListBoxRow| {
                update(model_ptr.clone(), &view, AppMessage::SampleSetSelected(uuid));
            }),
        );
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
                let sample = (*sample).clone();

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

                clicked.connect_pressed(clone!(@weak row => move |_: &GestureClick, _, _, _| {
                    row.activate();
                }));

                name_label.add_controller(clicked);

                let label_select = objects
                    .object::<gtk::DropDown>(format!("{row_index}-label-select"))
                    .unwrap();

                label_select.set_model(Some(&gtk::StringList::new(
                    &["(None)"]
                        .iter()
                        .chain(DRUM_LABELS.keys().iter())
                        .copied()
                        .collect::<Vec<_>>(),
                )));

                if let Ok(Some(label)) = set.get_label(&sample) {
                    set_dropdown_choice(&label_select, &DRUM_LABELS, &label);
                }

                label_select.connect_selected_item_notify(clone!(
                    @strong model_ptr,
                    @strong view,
                    @strong sample => move |e: &gtk::DropDown| {
                        if e.selected() > 0 {
                            update(
                                model_ptr.clone(),
                                &view,
                                AppMessage::SampleSetSampleLabelChanged(
                                    sample.clone(),
                                    Some(DRUM_LABELS[e.selected() as usize - 1].1)
                                )
                            );
                        } else {
                            update(
                                model_ptr.clone(),
                                &view,
                                AppMessage::SampleSetSampleLabelChanged(
                                    sample.clone(),
                                    None
                                )
                            );
                        }
                    }
                ));

                view.sets_details_sample_list.append(&row);

                if Some(&sample) == model.selected_set_member()
                    || (row_index == 0 && model.selected_set_member().is_none())
                {
                    row.activate();
                }

                row.connect_activate(clone!(
                    @strong model_ptr,
                    @strong view,
                    @strong sample => move |_: &gtk::ListBoxRow| {
                        update(
                            model_ptr.clone(),
                            &view,
                            AppMessage::SampleSetSampleSelected(sample.clone())
                        );
                    }
                ));
            }
        }
        None => {
            view.sets_details_name_label.set_text("");
        }
    }
}
