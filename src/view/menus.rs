use gtk::{gio::ActionEntry, glib::clone, prelude::*, Application};

use crate::{model::AppModelPtr, update, view::AsampoView, AppMessage};

pub fn build_actions(app: &Application, model_ptr: AppModelPtr, view: &AsampoView) {
    let action_open_savefile = ActionEntry::builder("open_savefile")
        .activate(
            clone!(@strong model_ptr, @strong view => move |_app: &Application, _, _| {
                let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
                let filter_json = gtk::FileFilter::new();

                filter_json.add_suffix("json");
                filters.append(&filter_json);

                let dialog = gtk::FileDialog::builder().modal(true).filters(&filters).build();

                dialog.open(
                    Some(&view),
                    None::<gtk::gio::Cancellable>.as_ref(),
                    clone!(@strong model_ptr, @strong view => move |result| {
                        match result {
                            Ok(gfile) => update(
                                model_ptr.clone(),
                                &view,
                                AppMessage::LoadFromSavefile(
                                    gfile
                                        .path()
                                        .unwrap()
                                        .into_os_string()
                                        .into_string()
                                        .unwrap()
                                )
                            ),

                            Err(e) => update(model_ptr.clone(), &view, AppMessage::DialogError(e)),
                        }
                    })
                );
            }),
        )
        .build();

    let action_save = ActionEntry::builder("save")
        .activate(
            clone!(@strong model_ptr, @strong view  => move |_app: &Application, _, _| {
                let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
                let filter_json = gtk::FileFilter::new();

                filter_json.add_suffix("json");
                filters.append(&filter_json);

                let model = model_ptr.take().unwrap();
                let initial_name = model.savefile.clone();
                model_ptr.set(Some(model));

                let mut dialog = gtk::FileDialog::builder().modal(true).filters(&filters);

                // TODO: separate path and basename
                if let Some(filename) = initial_name {
                    dialog = dialog.initial_name(filename);
                }

                dialog.build().save(
                    Some(&view),
                    None::<gtk::gio::Cancellable>.as_ref(),
                    clone!(@strong model_ptr, @strong view => move |result| {
                        match result {
                            Ok(gfile) => update(
                                model_ptr.clone(),
                                &view,
                                AppMessage::SaveToSavefile(
                                    gfile
                                        .path()
                                        .unwrap()
                                        .into_os_string()
                                        .into_string()
                                        .unwrap()
                                )
                            ),

                            Err(e) => update(model_ptr.clone(), &view, AppMessage::DialogError(e)),
                        }
                    })
                );
            }),
        )
        .build();

    app.add_action_entries([action_open_savefile, action_save]);
}
