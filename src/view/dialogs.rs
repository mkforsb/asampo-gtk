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
    let objects = gtk::Builder::from_string(indoc::indoc! {r#"
        <interface>
          <object class="GtkWindow">
            <child type="titlebar">
              <object class="GtkHeaderBar">
                <style>
                  <class name="less-tall" />
                </style>
                <property name="decoration-layout">:close</property>
                <property name="title-widget">
                  <object class="GtkLabel" id="title">
                    <property name="label"></property>
                    <property name="single-line-mode">true</property>
                    <style>
                      <class name="title" />
                    </style>
                  </object>
                </property>
              </object>
            </child>
            <child>
              <object class="GtkBox">
                <property name="orientation">GTK_ORIENTATION_VERTICAL</property>
                <child>
                  <object class="GtkBox">
                    <property name="orientation">GTK_ORIENTATION_HORIZONTAL</property>
                    <child>
                      <object class="GtkLabel" id="input-descr">
                        <property name="label"></property>
                      </object>
                    </child>
                    <child>
                      <object class="GtkEntry" id="input">
                        <property name="placeholder-text"></property>
                      </object>
                    </child>
                  </object>
                </child>
                <child>
                  <object class="GtkBox">
                    <property name="orientation">GTK_ORIENTATION_HORIZONTAL</property>
                    <child>
                      <object class="GtkButton" id="ok-button">
                        <property name="label"></property>
                      </object>
                    </child>
                    <child>
                      <object class="GtkButton" id="cancel-button">
                        <property name="label">Cancel</property>
                      </object>
                    </child>
                  </object>
                </child>
              </object>
            </child>
          </object>
        </interface>
    "#});

    let dialogwin = objects
        .object::<gtk::Window>("input-dialog-window")
        .unwrap();

    objects
        .object::<gtk::Label>("title")
        .unwrap()
        .set_text(title);

    objects
        .object::<gtk::Label>("input-descr")
        .unwrap()
        .set_text(input_descr);

    objects
        .object::<gtk::Entry>("input")
        .unwrap()
        .set_placeholder_text(Some(placeholder));

    let okbutton = objects.object::<gtk::Button>("ok-button").unwrap();
    okbutton.set_label(ok);

    let cancelbutton = objects.object::<gtk::Button>("cancel-button").unwrap();

    okbutton.connect_clicked(
        clone!(@strong model_ptr, @strong view, @strong dialogwin, @strong context => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::InputDialogSubmitted(
                context.clone(),
                util::gtk_find_child_by_builder_id(&dialogwin, "input")
                    .unwrap()
                    .dynamic_cast_ref::<gtk::Entry>()
                    .unwrap()
                    .text()
                    .to_string()
            ));

            view.set_sensitive(true);
            dialogwin.destroy();
        }),
    );

    cancelbutton.connect_clicked(
        clone!(@strong model_ptr, @strong view, @strong dialogwin, @strong context => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::InputDialogCanceled(context.clone()));
            view.set_sensitive(true);
            dialogwin.destroy();
        }),
    );

    dialogwin.connect_show(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Window| {
            view.set_sensitive(false);
            update(model_ptr.clone(), &view, AppMessage::InputDialogOpened);
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
}

#[derive(Debug, Clone)]
pub struct ExportDialogView {
    pub window: gtk::Window,
    pub target_dir_entry: gtk::Entry,
    pub export_button: gtk::Button,
}

pub fn sampleset_export(model_ptr: AppModelPtr, view: &AsampoView, model: AppModel) {
    let objects = gtk::Builder::from_string(indoc::indoc! {r#"
        <interface>
          <object class="GtkWindow" id="export-dialog-window">
            <child type="titlebar">
              <object class="GtkHeaderBar">
                <style>
                  <class name="less-tall" />
                </style>
                <property name="decoration-layout">:close</property>
                <property name="title-widget">
                  <object class="GtkLabel">
                    <property name="label">Export</property>
                    <property name="single-line-mode">true</property>
                    <style>
                      <class name="title" />
                    </style>
                  </object>
                </property>
              </object>
            </child>
            <child>
              <object class="GtkBox">
                <property name="orientation">GTK_ORIENTATION_VERTICAL</property>
                <child>
                  <object class="GtkBox">
                    <property name="orientation">GTK_ORIENTATION_HORIZONTAL</property>
                    <child>
                      <object class="GtkLabel">
                        <property name="label">Target directory:</property>
                      </object>
                    </child>
                    <child>
                      <object class="GtkEntry" id="target_dir_entry">
                        <property name="placeholder-text">/path/to/export</property>
                        <property name="hexpand">true</property>
                      </object>
                    </child>
                    <child>
                      <object class="GtkButton" id="browse_button">
                        <property name="label">Browse</property>
                      </object>
                    </child>
                  </object>
                </child>
                <child>
                  <object class="GtkBox">
                    <property name="orientation">GTK_ORIENTATION_HORIZONTAL</property>
                    <child>
                      <object class="GtkButton" id="export_button">
                        <property name="label">Export</property>
                        <property name="sensitive">false</property>
                      </object>
                    </child>
                    <child>
                      <object class="GtkButton" id="cancel_button">
                        <property name="label">Cancel</property>
                      </object>
                    </child>
                  </object>
                </child>
                <child>
                  <object class="GtkCheckButton" id="plain_copy_radio_button">
                    <property name="label">Plain copy</property>
                    <property name="active">true</property>
                  </object>
                </child>
                <child>
                  <object class="GtkCheckButton" id="convert_radio_button">
                    <property name="label">Convert</property>
                    <property name="group">plain_copy_radio_button</property>
                  </object>
                </child>
                <child>
                  <object class="GtkBox">
                    <property name="orientation">GTK_ORIENTATION_HORIZONTAL</property>
                    <child>
                      <object class="GtkDropDown" id="conversion_entry">
                        <property name="sensitive" bind-source="convert_radio_button" bind-property="active">false</property>
                        <property name="model">
                          <object class="GtkStringList">
                            <items>
                              <item>WAV 44.1 kHz 16-bit</item>
                            </items>
                          </object>
                        </property>
                      </object>
                    </child>
                    <child>
                      <object class="GtkButton">
                        <property name="label">Profiles ...</property>
                        <property name="sensitive">false</property>
                      </object>
                    </child>
                  </object>
                </child>
              </object>
            </child>
          </object>
        </interface>
    "#});

    let dialogwin = objects
        .object::<gtk::Window>("export-dialog-window")
        .unwrap();

    let target_dir_entry = objects.object::<gtk::Entry>("target_dir_entry").unwrap();
    let browse_button = objects.object::<gtk::Button>("browse_button").unwrap();
    let export_button = objects.object::<gtk::Button>("export_button").unwrap();
    let cancel_button = objects.object::<gtk::Button>("cancel_button").unwrap();

    let plain_copy_radio = objects
        .object::<gtk::CheckButton>("plain_copy_radio_button")
        .unwrap();

    let convert_radio = objects
        .object::<gtk::CheckButton>("convert_radio_button")
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
