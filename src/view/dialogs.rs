// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{glib::clone, prelude::*};

use crate::{model::AppModelPtr, update, view::AsampoView, AppMessage};

pub fn choose_folder(
    model_ptr: AppModelPtr,
    view: &AsampoView,
    ok: fn(String) -> AppMessage,
    err: fn(gtk::glib::Error) -> AppMessage,
) {
    let dialog = gtk::FileDialog::builder().modal(true).build();

    dialog.select_folder(
        Some(view),
        None::<gtk::gio::Cancellable>.as_ref(),
        clone!(@strong model_ptr, @strong view => move |result| {
            match result {
                Ok(gfile) => update(
                    model_ptr.clone(),
                    &view,
                    ok(gfile.path().unwrap().into_os_string().into_string().unwrap())
                ),

                Err(e) => update(model_ptr.clone(), &view, err(e)),
            }
        }),
    );
}
