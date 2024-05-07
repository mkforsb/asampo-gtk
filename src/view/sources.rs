// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{glib::clone, prelude::*, GestureClick};
use libasampo::prelude::*;

use crate::{update, view::AsampoView, AppMessage, AppModel, AppModelPtr};

pub fn setup_sources_page(model_ptr: AppModelPtr, view: &AsampoView) {
    view.sources_add_fs_name_entry.connect_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::Entry| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::AddFilesystemSourceNameChanged(e.text().to_string())
            );
        }),
    );

    view.sources_add_fs_path_entry.connect_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::Entry| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::AddFilesystemSourcePathChanged(e.text().to_string())
            );
        }),
    );

    view.sources_add_fs_path_browse_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::AddFilesystemSourcePathBrowseClicked);
        }),
    );

    view.sources_add_fs_extensions_entry.connect_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::Entry| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::AddFilesystemSourceExtensionsChanged(e.text().to_string())
            );
        }),
    );

    view.sources_add_fs_add_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::AddFilesystemSourceClicked);
        }),
    );
}

pub fn update_sources_list(model_ptr: AppModelPtr, model: AppModel, view: &AsampoView) {
    view.sources_list.remove_all();

    for uuid in model.sources_order.iter() {
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

        if model.sources.get(uuid).unwrap().is_enabled() {
            checkbutton.activate();
        }

        checkbutton.connect_toggled(
            clone!(@strong model_ptr, @strong uuid, @strong view => move |e: &gtk::CheckButton| {
                if e.is_active() {
                    update(model_ptr.clone(), &view, AppMessage::SourceEnabled(uuid))
                } else {
                    update(model_ptr.clone(), &view, AppMessage::SourceDisabled(uuid))
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
            .set_label(model.sources.get(uuid).unwrap().name().unwrap_or("Unnamed"));

        row.child()
            .unwrap()
            .dynamic_cast_ref::<gtk::Box>()
            .unwrap()
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .next_sibling()
            .unwrap()
            .dynamic_cast_ref::<gtk::Button>()
            .unwrap()
            .connect_clicked(
                clone!(@strong model_ptr, @strong view, @strong uuid => move |_: &gtk::Button| {
                    update(model_ptr.clone(), &view, AppMessage::SourceDeleteClicked(uuid));
                }),
            );

        let clicked = GestureClick::new();

        clicked.connect_pressed(|e: &GestureClick, _, _, _| {
            e.widget().activate();
        });

        row.add_controller(clicked);

        view.sources_list.append(row);
    }
}
