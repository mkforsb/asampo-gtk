// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

mod ext;
mod menus;
mod model;
mod samples;
mod savefile;
mod sources;
mod view;

use std::{cell::Cell, rc::Rc, sync::mpsc};

use anyhow::anyhow;
use gtk::{
    gio::ApplicationFlags,
    glib::{clone, ExitCode},
    prelude::*,
    Application,
};

use libasampo::{
    prelude::*,
    sources::{file_system_source::FilesystemSource, Source, SourceTrait},
};

use ext::WithModel;
use menus::build_actions;
use model::{AppFlags, AppModel, AppModelPtr, AppValues};
use samples::{setup_samples_page, SampleListEntry};
use savefile::Savefile;
use sources::setup_sources_page;
use sources::update_sources_list;
use uuid::Uuid;
use view::AsampoView;

#[derive(Debug)]
enum AppMessage {
    AddFilesystemSourceNameChanged(String),
    AddFilesystemSourcePathChanged(String),
    AddFilesystemSourceExtensionsChanged(String),
    AddFilesystemSourceClicked,
    SampleClicked(u32),
    SamplesFilterChanged(String),
    SourceEnabled(Uuid),
    SourceDisabled(Uuid),
    LoadFromSavefile(String),
    SaveToSavefile(String),
}

fn update(model_ptr: AppModelPtr, view: &AsampoView, message: AppMessage) {
    log::log!(log::Level::Debug, "{message:?}");

    let old_model = model_ptr.take().unwrap();

    match update_model(old_model.clone(), message) {
        Ok(new_model) => {
            model_ptr.set(Some(new_model.clone()));
            update_view(model_ptr, old_model, new_model, view);
        }

        Err(e) => {
            model_ptr.set(Some(old_model));
            log::log!(log::Level::Error, "{}", e.to_string());
        }
    }
}

fn update_model(model: AppModel, message: AppMessage) -> Result<AppModel, anyhow::Error> {
    fn check_sources_add_fs_valid(model: AppModel) -> AppModel {
        #[allow(clippy::needless_update)]
        AppModel {
            flags: AppFlags {
                sources_add_fs_fields_valid: !model.values.sources_add_fs_name_entry.is_empty()
                    && !model.values.sources_add_fs_path_entry.is_empty()
                    && !model.values.sources_add_fs_extensions_entry.is_empty(),
                ..model.flags
            },
            ..model
        }
    }

    match message {
        AppMessage::AddFilesystemSourceNameChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                values: AppValues {
                    sources_add_fs_name_entry: text,
                    ..model.values
                },
                ..model
            }))
        }

        AppMessage::AddFilesystemSourcePathChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                values: AppValues {
                    sources_add_fs_path_entry: text,
                    ..model.values
                },
                ..model
            }))
        }

        AppMessage::AddFilesystemSourceExtensionsChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                values: AppValues {
                    sources_add_fs_extensions_entry: text,
                    ..model.values
                },
                ..model
            }))
        }

        // TODO: more validation, e.g is the path readable
        AppMessage::AddFilesystemSourceClicked => {
            let new_source = Source::FilesystemSource(FilesystemSource::new_named(
                model.values.sources_add_fs_name_entry.clone(),
                model.values.sources_add_fs_path_entry.clone(),
                model
                    .values
                    .sources_add_fs_extensions_entry
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            ));

            let uuid = *new_source.uuid();
            let model = model.add_source(new_source).enable_source(uuid).unwrap();

            Ok(AppModel {
                #[allow(clippy::needless_update)]
                flags: AppFlags {
                    sources_add_fs_fields_valid: false,
                    ..model.flags
                },

                values: AppValues {
                    sources_add_fs_name_entry: String::from(""),
                    sources_add_fs_path_entry: String::from(""),
                    sources_add_fs_extensions_entry: String::from(""),
                    ..model.values
                },

                ..model
            }
            .map_ref(AppModel::populate_samples_listmodel))
        }

        AppMessage::SampleClicked(index) => {
            let item = model.samples_listview_model.item(index);

            match item
                .and_dynamic_cast_ref::<SampleListEntry>()
                .map(|x| &x.value)
            {
                Some(sample) => {
                    model.audiothread_tx.as_ref().unwrap().send(
                        audiothread::Message::PlaySymphoniaSource(
                            audiothread::SymphoniaSource::from_file(sample.borrow().uri())?,
                        ),
                    )?;

                    Ok(model)
                }
                None => Err(anyhow!("Could not obtain clicked sample (this is a bug)")),
            }
        }

        AppMessage::SamplesFilterChanged(text) => Ok(AppModel {
            values: AppValues {
                samples_list_filter: text,
                ..model.values
            },
            ..model
        }
        .map_ref(AppModel::populate_samples_listmodel)),

        AppMessage::SourceEnabled(uuid) => Ok(model
            .enable_source(uuid)?
            .map_ref(AppModel::populate_samples_listmodel)),

        AppMessage::SourceDisabled(uuid) => Ok(model
            .disable_source(uuid)?
            .map_ref(AppModel::populate_samples_listmodel)),

        AppMessage::LoadFromSavefile(filename) => {
            log::log!(log::Level::Info, "Loading from {filename}");

            match Savefile::load(&filename) {
                Ok(mut loaded_app_model) => {
                    loaded_app_model.audiothread_tx = model.audiothread_tx;
                    loaded_app_model._audiothread_handle = model._audiothread_handle;

                    loaded_app_model.load_enabled_sources()?;
                    loaded_app_model.populate_samples_listmodel();

                    Ok(loaded_app_model)
                }
                Err(e) => Err(e),
            }
        }

        AppMessage::SaveToSavefile(filename) => {
            log::log!(log::Level::Info, "Saving to {filename}");

            match Savefile::save(&model, &filename) {
                Ok(_) => Ok(AppModel {
                    savefile: Some(filename),
                    ..model
                }),

                Err(e) => Err(e),
            }
        }
    }
}

fn update_view(model_ptr: AppModelPtr, old: AppModel, new: AppModel, view: &AsampoView) {
    macro_rules! maybe_update_entry_text {
        ($old:ident, $new:ident, $view:ident, $entry:ident) => {
            if $old.values.$entry != $new.values.$entry && $view.$entry.text() != $new.values.$entry
            {
                $view.$entry.set_text(&$new.values.$entry);
            }
        };
    }

    maybe_update_entry_text!(old, new, view, sources_add_fs_name_entry);
    maybe_update_entry_text!(old, new, view, sources_add_fs_path_entry);
    maybe_update_entry_text!(old, new, view, sources_add_fs_extensions_entry);

    if old.flags.sources_add_fs_fields_valid != new.flags.sources_add_fs_fields_valid {
        view.sources_add_fs_add_button
            .set_sensitive(new.flags.sources_add_fs_fields_valid);
    }

    if old.sources != new.sources {
        update_sources_list(model_ptr, new.clone(), view);
    }

    if old.samples_listview_model != new.samples_listview_model {
        view.samples_listview
            .set_model(Some(&gtk::SingleSelection::new(Some(
                new.samples_listview_model.clone(),
            ))));
    }
}

fn main() -> ExitCode {
    env_logger::init();

    gtk::gio::resources_register_include!("resources.gresource")
        .expect("Should be able to register compiled GTK resources.");

    let app = Application::builder()
        .application_id("se.neode.Asampo")
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_command_line(clone!(@strong app =>  move |_, _| {
        app.activate();
        0
    }));

    app.connect_activate(|app| {
        let (tx, rx) = mpsc::channel();
        let audiothread_handle = Rc::new(audiothread::spawn(
            rx,
            Some(
                audiothread::Opts::default()
                    .with_name("Asampo")
                    .with_bufsize_n_stereo_samples(1024),
            ),
        ));

        let view = AsampoView::new(app);

        let model = AppModel::new(None, Some(tx.clone()), Some(audiothread_handle.clone()));
        let model_ptr = Rc::new(Cell::new(Some(model.clone())));

        setup_sources_page(model_ptr.clone(), &view);
        setup_samples_page(model_ptr.clone(), &view);

        build_actions(app, model_ptr, &view);

        view.present();
    });

    app.run()
}
