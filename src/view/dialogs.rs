// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{
    glib::{clone, Propagation},
    prelude::*,
};

use crate::{
    model::{AppModel, AppModelPtr},
    update, util,
    view::AsampoView,
    AppMessage, InputDialogContext, SelectFolderDialogContext,
};

pub fn choose_folder(
    model_ptr: AppModelPtr,
    view: &AsampoView,
    context: SelectFolderDialogContext,
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

    update(
        model_ptr.clone(),
        view,
        AppMessage::SelectFolderDialogOpened(context),
    );
}

pub fn alert(_model_ptr: AppModelPtr, view: &AsampoView, message: &str, detail: &str) {
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message(message)
        .detail(detail)
        .build();

    dialog.show(Some(view));
}

pub fn input(
    model_ptr: AppModelPtr,
    view: &AsampoView,
    context: InputDialogContext,
    title: &str,
    input_descr: &str,
    placeholder: &str,
    ok: &str,
) {
    let objects = gtk::Builder::from_resource("/input-dialog.ui");

    let dialogwin = objects
        .object::<gtk::Window>("input-dialog-window")
        .unwrap();

    objects
        .object::<gtk::Label>("title")
        .unwrap()
        .set_text(title);

    objects
        .object::<gtk::Label>("input-description")
        .unwrap()
        .set_text(input_descr);

    let input = objects.object::<gtk::Entry>("input").unwrap();
    input.set_placeholder_text(Some(placeholder));

    let okbutton = objects.object::<gtk::Button>("ok-button").unwrap();
    okbutton.set_label(ok);

    let cancelbutton = objects.object::<gtk::Button>("cancel-button").unwrap();

    okbutton.connect_clicked(
        clone!(@strong model_ptr, @strong view, @strong dialogwin, @strong context => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::InputDialogSubmitted(
                context.clone(),
                util::gtk_find_child_by_builder_id::<gtk::Entry>(&dialogwin, "input")
                    .unwrap()
                    .text()
                    .to_string()
            ));

            view.set_sensitive(true);
            dialogwin.destroy();
        }),
    );

    input.connect_activate(clone!(@strong okbutton => move |_| {
        okbutton.emit_clicked();
    }));

    cancelbutton.connect_clicked(
        clone!(@strong model_ptr, @strong view, @strong dialogwin, @strong context => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::InputDialogCanceled(context.clone()));
            view.set_sensitive(true);
            dialogwin.destroy();
        }),
    );

    dialogwin.connect_show(
        clone!(@strong model_ptr, @strong view, @strong context => move |_: &gtk::Window| {
            view.set_sensitive(false);
            update(model_ptr.clone(), &view, AppMessage::InputDialogOpened(context.clone()));
        }),
    );

    dialogwin.connect_close_request(
        clone!(@strong model_ptr, @strong view, @strong context => move |_: &gtk::Window| {
            update(model_ptr.clone(), &view, AppMessage::InputDialogCanceled(context.clone()));
            view.set_sensitive(true);
            Propagation::Proceed
        }),
    );

    dialogwin.set_modal(true);
    dialogwin.set_transient_for(Some(view));
    dialogwin.present();

    input.grab_focus();
}

#[derive(Debug, Clone)]
pub struct ExportDialogView {
    pub window: gtk::Window,
    pub target_dir_entry: gtk::Entry,
    pub export_button: gtk::Button,
}

pub fn sampleset_export(model_ptr: AppModelPtr, view: &AsampoView, model: AppModel) {
    let objects = gtk::Builder::from_resource("/export-dialog.ui");

    let dialogwin = objects
        .object::<gtk::Window>("export-dialog-window")
        .unwrap();

    let target_dir_entry = objects
        .object::<gtk::Entry>("target-directory-entry")
        .unwrap();

    let browse_button = objects.object::<gtk::Button>("browse-button").unwrap();
    let export_button = objects.object::<gtk::Button>("export-button").unwrap();
    let cancel_button = objects.object::<gtk::Button>("cancel-button").unwrap();

    let plain_copy_radio = objects
        .object::<gtk::CheckButton>("plain-copy-radio-button")
        .unwrap();

    let convert_radio = objects
        .object::<gtk::CheckButton>("convert-radio-button")
        .unwrap();

    target_dir_entry.set_text(&model.viewvalues.sets_export_target_dir_entry);
    export_button.set_sensitive(target_dir_entry.text_length() > 0);

    match model.viewvalues.sets_export_kind {
        Some(crate::model::ExportKind::PlainCopy) => {
            plain_copy_radio.set_active(true);
            convert_radio.set_active(false);
        }

        Some(crate::model::ExportKind::Conversion) => {
            plain_copy_radio.set_active(false);
            convert_radio.set_active(true);
        }

        None => (),
    }

    target_dir_entry.connect_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::Entry| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::ExportTargetDirectoryChanged(e.text().to_string())
            );
        }),
    );

    browse_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::ExportTargetDirectoryBrowseClicked);
        }),
    );

    export_button.connect_clicked(
        clone!(@strong model_ptr, @strong view, @strong dialogwin => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::PerformExportClicked);
        }),
    );

    cancel_button.connect_clicked(
        clone!(@strong model_ptr, @strong view, @strong dialogwin => move |_: &gtk::Button| {
            dialogwin.close()
        }),
    );

    plain_copy_radio.connect_toggled(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::CheckButton| {
            if e.is_active() {
                update(model_ptr.clone(), &view, AppMessage::PlainCopyExportSelected);
            }
        }),
    );

    convert_radio.connect_toggled(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::CheckButton| {
            if e.is_active() {
                update(model_ptr.clone(), &view, AppMessage::ConversionExportSelected);
            }
        }),
    );

    dialogwin.connect_close_request(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Window| {
            update(model_ptr.clone(), &view, AppMessage::ExportDialogClosed);
            Propagation::Proceed
        }),
    );

    dialogwin.set_modal(true);
    dialogwin.set_transient_for(Some(view));
    dialogwin.present();

    update(
        model_ptr.clone(),
        view,
        AppMessage::ExportDialogOpened(ExportDialogView {
            window: dialogwin.clone(),
            target_dir_entry: target_dir_entry.clone(),
            export_button: export_button.clone(),
        }),
    );
}
