// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{
    glib::{self, clone, Propagation},
    prelude::*,
    EventControllerKey,
};

use crate::{
    model::{AppModel, AppModelPtr},
    update, util,
    view::AsampoView,
    AppMessage,
};

#[derive(Debug, Clone)]
pub enum InputDialogContext {
    AddToSampleset,
    CreateSampleSet,
    CreateEmptySequence,
    SaveDrumMachineSequenceAs,
    SaveDrumMachineSampleSetAs,
}

#[derive(Debug, Clone)]
pub enum SelectFolderDialogContext {
    BrowseForFilesystemSource,
    BrowseForExportTargetDirectory,
}

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

    okbutton.connect_clicked(clone!(
        @strong model_ptr,
        @strong view,
        @strong dialogwin,
        @strong context => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::InputDialogSubmitted(
                context.clone(),
                util::gtk_find_child_by_builder_id::<gtk::Entry>(&dialogwin, "input")
                    .unwrap()
                    .text()
                    .to_string()
            ));

            view.set_sensitive(true);
            dialogwin.destroy();
        }
    ));

    input.connect_activate(clone!(@strong okbutton => move |_| {
        okbutton.emit_clicked();
    }));

    cancelbutton.connect_clicked(clone!(
        @strong model_ptr,
        @strong view,
        @strong dialogwin,
        @strong context => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::InputDialogCanceled(context.clone()));
            view.set_sensitive(true);
            dialogwin.destroy();
        }
    ));

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

    let key_ctrl = EventControllerKey::new();
    key_ctrl.connect_key_released(clone!(@weak dialogwin => move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            dialogwin.close();
        }
    }));

    dialogwin.add_controller(key_ctrl);

    dialogwin.set_modal(true);
    dialogwin.set_transient_for(Some(view));
    dialogwin.present();

    input.grab_focus();
}

#[derive(Debug, Clone)]
pub struct ExportDialogView {
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

    target_dir_entry.set_text(model.export_target_dir());
    export_button.set_sensitive(target_dir_entry.text_length() > 0);

    match model.export_kind() {
        crate::model::ExportKind::PlainCopy => {
            plain_copy_radio.set_active(true);
            convert_radio.set_active(false);
        }

        crate::model::ExportKind::Conversion => {
            plain_copy_radio.set_active(false);
            convert_radio.set_active(true);
        }
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

    export_button.connect_clicked(clone!(
        @weak dialogwin,
        @strong model_ptr,
        @strong view,
        @strong dialogwin => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::PerformExportClicked);
            dialogwin.close()
        }
    ));

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
            update(
                model_ptr.clone(),
                &view,
                AppMessage::Sequence(vec![
                    AppMessage::DialogClosed,
                    AppMessage::ExportDialogClosed
                ])
            );
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
            target_dir_entry: target_dir_entry.clone(),
            export_button: export_button.clone(),
        }),
    );
}

#[derive(Debug)]
pub struct ButtonSpec {
    pub text: String,
    pub action: fn() -> AppMessage,
    pub is_default: bool,
    pub is_cancel: bool,
}

impl ButtonSpec {
    pub fn new(text: impl Into<String>, action: fn() -> AppMessage) -> ButtonSpec {
        ButtonSpec {
            text: text.into(),
            action,
            is_default: false,
            is_cancel: false,
        }
    }

    pub fn set_as_default(self) -> ButtonSpec {
        ButtonSpec {
            is_default: true,
            ..self
        }
    }

    pub fn set_as_cancel(self) -> ButtonSpec {
        ButtonSpec {
            is_cancel: true,
            ..self
        }
    }
}

pub fn confirm(
    model_ptr: AppModelPtr,
    view: &AsampoView,
    message: &str,
    detail: &str,
    buttons: Vec<ButtonSpec>,
    on_open: AppMessage,
    err: fn(gtk::glib::Error) -> AppMessage,
) {
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message(message)
        .detail(detail)
        .buttons(
            buttons
                .iter()
                .map(|but| but.text.as_str())
                .collect::<Vec<_>>(),
        )
        .default_button(
            buttons
                .iter()
                .position(|but| but.is_default)
                .map(|x| x as i32)
                .unwrap_or(-1),
        )
        .cancel_button(
            buttons
                .iter()
                .position(|but| but.is_cancel)
                .map(|x| x as i32)
                .unwrap_or(-1),
        )
        .build();

    dialog.choose(
        Some(view),
        None::<&gtk::gio::Cancellable>,
        clone!(@strong model_ptr, @strong view => move |result: Result<i32, gtk::glib::Error>| {
            match result {
                Ok(n) if n >= 0 && n < buttons.len() as i32 => {
                    update(model_ptr.clone(), &view, (buttons[n as usize].action)());
                }

                Ok(n) => {
                    log::log!(
                        log::Level::Error,
                        "Unexpected index returned in confirm dialog: {n}"
                    );
                }

                Err(e) => update(model_ptr.clone(), &view, err(e)),
            }
        }),
    );

    update(model_ptr.clone(), view, on_open);
}

pub fn save(
    model_ptr: AppModelPtr,
    view: &AsampoView,
    on_open: AppMessage,
    ok: fn(String) -> AppMessage,
    err: fn(gtk::glib::Error) -> AppMessage,
) {
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    let filter_json = gtk::FileFilter::new();

    filter_json.add_suffix("json");
    filters.append(&filter_json);

    let model = model_ptr.take().unwrap();
    let maybe_initial_name = model.savefile_path().cloned();
    model_ptr.set(Some(model));

    let mut dialog = gtk::FileDialog::builder().modal(true).filters(&filters);

    // TODO: separate path and basename
    if let Some(filename) = maybe_initial_name {
        dialog = dialog.initial_name(filename);
    } else {
        dialog = dialog.initial_name("workspace.json");
    }

    dialog.build().save(
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

    update(model_ptr.clone(), view, on_open);
}

pub fn open(
    model_ptr: AppModelPtr,
    view: &AsampoView,
    on_open: AppMessage,
    ok: fn(String) -> AppMessage,
    err: fn(gtk::glib::Error) -> AppMessage,
) {
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    let filter_json = gtk::FileFilter::new();

    filter_json.add_suffix("json");
    filters.append(&filter_json);

    let dialog = gtk::FileDialog::builder()
        .modal(true)
        .filters(&filters)
        .build();

    dialog.open(
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

    update(model_ptr.clone(), view, on_open);
}
