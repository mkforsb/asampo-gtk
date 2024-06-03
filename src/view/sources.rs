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
        let objects = gtk::Builder::from_string(&indoc::formatdoc! {r#"
            <interface>
                <object class="GtkListBoxRow" id="{uuid}-row">
                    <child>
                        <object class="GtkBox">
                            <property name="orientation">GTK_ORIENTATION_HORIZONTAL</property>
                            <child>
                                <object class="GtkCheckButton" id="{uuid}-enable-checkbutton">
                                    <property name="margin-top">10</property>
                                    <property name="margin-start">10</property>
                                    <property name="margin-bottom">10</property>
                                    <property name="tooltip-text">Enable?</property>
                                </object>
                            </child>
                            <child>
                                <object class="GtkLabel" id="{uuid}-name-label">
                                    <property name="label"></property>
                                    <property name="xalign">0.0</property>
                                    <property name="margin_start">10</property>
                                    <property name="margin_top">10</property>
                                    <property name="margin_bottom">10</property>
                                </object>
                            </child>
                            <child>
                                <object class="GtkLabel" id="{uuid}-count-label">
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
                                <object class="GtkButton" id="{uuid}-delete-button">
                                    <property name="label">Delete</property>
                                    <property name="margin_end">16</property>
                                </object>
                            </child>
                        </object>
                    </child>
                </object>
            </interface>
        "#});

        let row = objects
            .object::<gtk::ListBoxRow>(&format!("{uuid}-row"))
            .unwrap();

        let enable_checkbutton = objects
            .object::<gtk::CheckButton>(&format!("{uuid}-enable-checkbutton"))
            .unwrap();

        let name_label = objects
            .object::<gtk::Label>(&format!("{uuid}-name-label"))
            .unwrap();

        let delete_button = objects
            .object::<gtk::Button>(&format!("{uuid}-delete-button"))
            .unwrap();

        if model.sources.get(uuid).unwrap().is_enabled() {
            enable_checkbutton.activate();
        }

        enable_checkbutton.connect_toggled(
            clone!(@strong model_ptr, @strong uuid, @strong view => move |e: &gtk::CheckButton| {
                if e.is_active() {
                    update(model_ptr.clone(), &view, AppMessage::SourceEnabled(uuid))
                } else {
                    update(model_ptr.clone(), &view, AppMessage::SourceDisabled(uuid))
                }
            }),
        );

        name_label.set_label(model.sources.get(uuid).unwrap().name().unwrap_or("Unnamed"));

        delete_button.connect_clicked(
            clone!(@strong model_ptr, @strong view, @strong uuid => move |_: &gtk::Button| {
                update(model_ptr.clone(), &view, AppMessage::SourceDeleteClicked(uuid));
            }),
        );

        let clicked = GestureClick::new();

        clicked.connect_pressed(|e: &GestureClick, _, _, _| {
            e.widget().activate();
        });

        row.add_controller(clicked);

        view.sources_list.append(&row);
    }
}
