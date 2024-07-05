// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{glib::clone, prelude::*, GestureClick};
use libasampo::prelude::*;

use crate::{
    update,
    util::{resource_as_string, uuidize_builder_template},
    view::AsampoView,
    AppMessage, AppModel, AppModelPtr,
};

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

    for source in model.sources_list().iter() {
        let uuid = source.uuid();

        let objects = gtk::Builder::from_string(&uuidize_builder_template(
            &resource_as_string("/sources-list-row.ui").unwrap(),
            *uuid,
        ));

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

        if model.source(*uuid).unwrap().is_enabled() {
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

        name_label.set_label(model.source(*uuid).unwrap().name().unwrap_or("Unnamed"));

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
