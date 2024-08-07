// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{gio::ActionEntry, glib::clone, prelude::*, Application};

use crate::{dialogs, model::AppModelPtr, view::AsampoView, AppMessage};

pub fn build_actions(app: &Application, model_ptr: AppModelPtr, view: &AsampoView) {
    let action_open_savefile = ActionEntry::builder("open_savefile")
        .activate(
            clone!(@strong model_ptr, @strong view => move |_app: &Application, _, _| {
                dialogs::open(
                    model_ptr.clone(),
                    &view,
                    AppMessage::NoOp,
                    AppMessage::LoadFromSavefile,
                    |e| AppMessage::LogError(anyhow::Error::from(e).into())
                );
            }),
        )
        .build();

    let action_save = ActionEntry::builder("save")
        .activate(
            clone!(@strong model_ptr, @strong view  => move |_app: &Application, _, _| {
                dialogs::save(
                    model_ptr.clone(),
                    &view,
                    AppMessage::NoOp,
                    AppMessage::SaveToSavefile,
                    |e| AppMessage::LogError(anyhow::Error::from(e).into())
                );
            }),
        )
        .build();

    app.add_action_entries([action_open_savefile, action_save]);
}
