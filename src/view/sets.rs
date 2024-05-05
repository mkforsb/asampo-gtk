// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)
use gtk::{glib::clone, prelude::*};

use crate::{model::AppModelPtr, update, view::AsampoView, AppMessage};

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
