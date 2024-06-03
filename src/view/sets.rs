// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{glib::clone, prelude::*, EventControllerKey, GestureClick};
use libasampo::{
    samples::SampleOps,
    samplesets::{SampleSetLabelling, SampleSetOps},
};

use crate::{
    ext::OptionMapExt,
    model::{AppModel, AppModelPtr},
    update,
    util::{set_dropdown_choice, strs_dropdown_get_selected},
    view::AsampoView,
    AppMessage,
};

#[derive(Debug, Clone, PartialEq)]
pub enum LabellingKind {
    None,
    Drumkit,
}

pub const LABELLING_OPTIONS: [(&str, LabellingKind); 2] = [
    ("None", LabellingKind::None),
    ("Drumkit", LabellingKind::Drumkit),
];

pub fn setup_sets_page(model_ptr: AppModelPtr, view: &AsampoView) {
    let labelling_model = gtk::StringList::new(&LABELLING_OPTIONS.keys());

    view.sets_detail_labelling_kind_entry
        .set_model(Some(&labelling_model));

    view.sets_detail_labelling_kind_entry
        .connect_selected_item_notify(
            clone!(@strong model_ptr, @strong view => move |e: &gtk::DropDown| {
                let kind = LABELLING_OPTIONS
                    .value_for(&strs_dropdown_get_selected(e))
                    .expect("Key should be valid");

                update(
                    model_ptr.clone(),
                    &view,
                    AppMessage::SampleSetLabellingKindChanged(kind.clone())
                );
            }),
        );

    view.sets_detail_export_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::SampleSetDetailsExportClicked);
        }),
    );
}

pub fn update_samplesets_list(model_ptr: AppModelPtr, model: AppModel, view: &AsampoView) {
    view.sets_list.remove_all();

    for uuid in model.sets_order.iter() {
        let objects = gtk::Builder::from_string(indoc::indoc! {r#"
            <interface>
              <object class="GtkListBoxRow">
                <child>
                  <object class="GtkLabel">
                    <property name="halign">GTK_ALIGN_FILL</property>
                    <property name="hexpand">true</property>
                    <property name="xalign">0.0</property>
                  </object>
                </child>
              </object>
            </interface>
        "#})
        .objects();

        let row = objects[0].dynamic_cast_ref::<gtk::ListBoxRow>().unwrap();

        let label_raw = row.child().unwrap();
        let label = label_raw.dynamic_cast_ref::<gtk::Label>().unwrap();

        label.set_text(model.sets.get(uuid).unwrap().name());

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

        view.sets_list.append(row);
    }
}

pub fn update_samplesets_detail(model_ptr: AppModelPtr, model: AppModel, view: &AsampoView) {
    view.sets_detail_sample_list.remove_all();

    match model
        .sets_selected_set
        .and_then(|uuid| model.sets.get(&uuid))
    {
        Some(set) => {
            view.sets_detail_name_label.set_text(set.name());

            set_dropdown_choice(
                &view.sets_detail_labelling_kind_entry,
                &LABELLING_OPTIONS,
                &match set.labelling() {
                    Some(SampleSetLabelling::DrumkitLabelling(_)) => LabellingKind::Drumkit,
                    None => LabellingKind::None,
                },
            );

            for (row_index, sample) in set.list().iter().enumerate() {
                view.sets_detail_sample_list
                    .append(&gtk::Label::builder().label(sample.name()).build());

                let row = view
                    .sets_detail_sample_list
                    .row_at_index(row_index.try_into().unwrap())
                    .unwrap();

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
            }
        }
        None => {
            view.sets_detail_name_label.set_text("");
        }
    }
}
