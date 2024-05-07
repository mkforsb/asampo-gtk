// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)
use gtk::{glib::clone, prelude::*, GestureClick};
use libasampo::samplesets::SampleSetOps;

use crate::{
    model::{AppModel, AppModelPtr},
    update,
    view::AsampoView,
    AppMessage,
};

pub fn setup_sets_page(model_ptr: AppModelPtr, view: &AsampoView) {
    view.samplesets_add_name_entry.connect_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::Entry| {
            update(model_ptr.clone(), &view, AppMessage::AddSampleSetNameChanged(e.text().to_string()));
        }),
    );

    view.samplesets_add_add_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_e: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::AddSampleSetClicked());
        }),
    );
}

pub fn update_samplesets_list(_model_ptr: AppModelPtr, model: AppModel, view: &AsampoView) {
    view.samplesets_list.remove_all();

    for uuid in model.samplesets_order.iter() {
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

        label.set_text(model.samplesets.get(uuid).unwrap().name());

        let clicked = GestureClick::new();

        clicked.connect_pressed(|e: &GestureClick, _, _, _| {
            e.widget().activate();
        });

        row.add_controller(clicked);

        view.samplesets_list.append(row);
    }
}
