// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{
    glib::{clone, Propagation},
    prelude::*,
};

use crate::{
    model::AppModelPtr, update, util, view::AsampoView, AppMessage, InputDialogContext,
    SelectFolderDialogContext,
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
    "#})
    .objects();

    let dialogwin = objects
        .iter()
        .find(|element| element.dynamic_cast_ref::<gtk::Window>().is_some());

    let dialogwin = dialogwin
        .unwrap()
        .dynamic_cast_ref::<gtk::Window>()
        .unwrap();

    let title_label_raw = util::gtk_find_child_by_builder_id(dialogwin, "title").unwrap();
    let title_label = title_label_raw.dynamic_cast_ref::<gtk::Label>().unwrap();
    title_label.set_text(title);

    let descr_label_raw = util::gtk_find_child_by_builder_id(dialogwin, "input-descr").unwrap();
    let descr_label = descr_label_raw.dynamic_cast_ref::<gtk::Label>().unwrap();
    descr_label.set_text(input_descr);

    let input_raw = util::gtk_find_child_by_builder_id(dialogwin, "input").unwrap();
    let input = input_raw.dynamic_cast_ref::<gtk::Entry>().unwrap();
    input.set_placeholder_text(Some(placeholder));

    let okbutton_raw = util::gtk_find_child_by_builder_id(dialogwin, "ok-button").unwrap();
    let okbutton = okbutton_raw.dynamic_cast_ref::<gtk::Button>().unwrap();
    okbutton.set_label(ok);

    let cancelbutton_raw = util::gtk_find_child_by_builder_id(dialogwin, "cancel-button").unwrap();
    let cancelbutton = cancelbutton_raw.dynamic_cast_ref::<gtk::Button>().unwrap();

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
