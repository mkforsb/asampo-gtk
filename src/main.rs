// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

mod config;
mod configfile;
mod ext;
mod model;
mod savefile;
mod view;

use std::{cell::Cell, io::BufReader, rc::Rc, sync::mpsc};

use anyhow::anyhow;
use config::AppConfig;
use configfile::ConfigFile;
use gtk::{
    gdk::Display,
    gio::ApplicationFlags,
    glib::{clone, ExitCode},
    prelude::*,
    Application, DialogError,
};
use uuid::Uuid;

use libasampo::{
    prelude::*,
    sources::{file_system_source::FilesystemSource, Source},
};

use config::OptionMapExt;
use ext::WithModel;
use model::{AppModel, AppModelPtr, ViewFlags, ViewValues};
use savefile::Savefile;
use view::{
    menus::build_actions,
    samples::{setup_samples_page, SampleListEntry},
    settings::setup_settings_page,
    sources::{setup_sources_page, update_sources_list},
    AsampoView,
};

#[derive(Debug)]
enum AppMessage {
    SettingsOutputSampleRateChanged(String),
    SettingsBufferSizeChanged(u16),
    SettingsSampleRateConversionQualityChanged(String),
    SettingsSamplePlaybackBehaviorChanged(String),
    AddFilesystemSourceNameChanged(String),
    AddFilesystemSourcePathChanged(String),
    AddFilesystemSourcePathBrowseClicked,
    AddFilesystemSourcePathBrowseSubmitted(String),
    AddFilesystemSourcePathBrowseError(gtk::glib::Error),
    AddFilesystemSourceExtensionsChanged(String),
    AddFilesystemSourceClicked,
    SampleClicked(u32),
    SamplesFilterChanged(String),
    SourceEnabled(Uuid),
    SourceDisabled(Uuid),
    SourceDeleteClicked(Uuid),
    LoadFromSavefile(String),
    SaveToSavefile(String),
    DialogError(gtk::glib::Error),
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
            viewflags: ViewFlags {
                sources_add_fs_fields_valid: !model.viewvalues.sources_add_fs_name_entry.is_empty()
                    && !model.viewvalues.sources_add_fs_path_entry.is_empty()
                    && !model.viewvalues.sources_add_fs_extensions_entry.is_empty(),
                ..model.viewflags
            },
            ..model
        }
    }

    match message {
        AppMessage::SettingsOutputSampleRateChanged(choice) => {
            let new_config = AppConfig {
                output_samplerate_hz: match config::OUTPUT_SAMPLE_RATE_OPTIONS.value_for(&choice) {
                    Some(value) => *value,
                    None => {
                        log::log!(
                            log::Level::Error,
                            "Unknown output sample rate setting, using default"
                        );
                        AppConfig::default().output_samplerate_hz
                    }
                },
                ..model.config.expect("There should be an active config")
            };

            let settings_latency_approx_label = new_config.fmt_latency_approx();

            Ok(AppModel {
                config: Some(new_config),
                viewvalues: ViewValues {
                    settings_latency_approx_label,
                    ..model.viewvalues
                },
                ..model
            })
        }

        AppMessage::SettingsBufferSizeChanged(samples) => {
            let new_config = AppConfig {
                buffer_size_samples: samples,
                ..model.config.expect("There should be an active config")
            };

            let settings_latency_approx_label = new_config.fmt_latency_approx();

            Ok(AppModel {
                config: Some(new_config),
                viewvalues: ViewValues {
                    settings_latency_approx_label,
                    ..model.viewvalues
                },
                ..model
            })
        }

        AppMessage::SettingsSampleRateConversionQualityChanged(choice) => Ok(AppModel {
            config: Some(AppConfig {
                sample_rate_conversion_quality: match config::SAMPLE_RATE_CONVERSION_QUALITY_OPTIONS
                    .value_for(&choice)
                {
                    Some(value) => value.clone(),
                    None => {
                        log::log!(
                            log::Level::Error,
                            "Unknown sample rate conversion quality setting, using default"
                        );
                        AppConfig::default().sample_rate_conversion_quality
                    }
                },
                ..model.config.expect("There should be an active config")
            }),
            ..model
        }),

        AppMessage::SettingsSamplePlaybackBehaviorChanged(choice) => Ok(AppModel {
            config: Some(AppConfig {
                sample_playback_behavior: match config::SAMPLE_PLAYBACK_BEHAVIOR_OPTIONS
                    .value_for(&choice)
                {
                    Some(value) => value.clone(),
                    None => {
                        log::log!(
                            log::Level::Error,
                            "Unknown sample playback behavior setting, using default"
                        );
                        AppConfig::default().sample_playback_behavior
                    }
                },
                ..model.config.expect("There should be an active config")
            }),
            ..model
        }),

        AppMessage::AddFilesystemSourceNameChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                viewvalues: ViewValues {
                    sources_add_fs_name_entry: text,
                    ..model.viewvalues
                },
                ..model
            }))
        }

        AppMessage::AddFilesystemSourcePathChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                viewvalues: ViewValues {
                    sources_add_fs_path_entry: text,
                    ..model.viewvalues
                },
                ..model
            }))
        }

        AppMessage::AddFilesystemSourcePathBrowseClicked => Ok(AppModel {
            viewflags: ViewFlags {
                sources_add_fs_browse: true,
                ..model.viewflags
            },
            ..model
        }),

        AppMessage::AddFilesystemSourcePathBrowseSubmitted(text) => Ok(AppModel {
            viewflags: ViewFlags {
                sources_add_fs_browse: false,
                ..model.viewflags
            },
            viewvalues: ViewValues {
                sources_add_fs_name_entry: if model.viewvalues.sources_add_fs_name_entry.is_empty()
                {
                    if let Some(name) = std::path::Path::new(&text)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                    {
                        name
                    } else {
                        model.viewvalues.sources_add_fs_name_entry
                    }
                } else {
                    model.viewvalues.sources_add_fs_name_entry
                },
                sources_add_fs_path_entry: text,
                ..model.viewvalues
            },
            ..model
        }),

        AppMessage::AddFilesystemSourcePathBrowseError(error) => {
            log::log!(log::Level::Debug, "Error browsing for folder: {error:?}");

            Ok(AppModel {
                viewflags: ViewFlags {
                    sources_add_fs_browse: false,
                    ..model.viewflags
                },
                ..model
            })
        }

        AppMessage::AddFilesystemSourceExtensionsChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                viewvalues: ViewValues {
                    sources_add_fs_extensions_entry: text,
                    ..model.viewvalues
                },
                ..model
            }))
        }

        // TODO: more validation, e.g is the path readable
        AppMessage::AddFilesystemSourceClicked => {
            let new_source = Source::FilesystemSource(FilesystemSource::new_named(
                model.viewvalues.sources_add_fs_name_entry.clone(),
                model.viewvalues.sources_add_fs_path_entry.clone(),
                model
                    .viewvalues
                    .sources_add_fs_extensions_entry
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            ));

            let uuid = *new_source.uuid();
            let model = model.add_source(new_source).enable_source(uuid).unwrap();

            Ok(AppModel {
                #[allow(clippy::needless_update)]
                viewflags: ViewFlags {
                    sources_add_fs_fields_valid: false,
                    ..model.viewflags
                },

                viewvalues: ViewValues {
                    sources_add_fs_name_entry: String::from(""),
                    sources_add_fs_path_entry: String::from(""),
                    sources_add_fs_extensions_entry: String::from(""),
                    ..model.viewvalues
                },

                ..model
            }
            .map_ref(AppModel::populate_samples_listmodel))
        }

        AppMessage::SampleClicked(index) => {
            let item = model.viewvalues.samples_listview_model.item(index);

            match item
                .and_dynamic_cast_ref::<SampleListEntry>()
                .map(|x| &x.value)
            {
                Some(sample) => {
                    let stream = model
                        .sources
                        .get(
                            sample
                                .borrow()
                                .source_uuid()
                                .ok_or(anyhow!("Sample missing source uuid"))?,
                        )
                        .ok_or(anyhow!("Failed to get source for sample"))?
                        .stream(&sample.borrow())?;

                    model.audiothread_tx.as_ref().unwrap().send(
                        audiothread::Message::PlaySymphoniaSource(
                            audiothread::SymphoniaSource::from_buf_reader(BufReader::new(stream))?,
                        ),
                    )?;

                    Ok(model)
                }
                None => Err(anyhow!("Could not obtain clicked sample (this is a bug)")),
            }
        }

        AppMessage::SamplesFilterChanged(text) => Ok(AppModel {
            viewvalues: ViewValues {
                samples_list_filter: text,
                ..model.viewvalues
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

        AppMessage::SourceDeleteClicked(uuid) => Ok(model
            .remove_source(uuid)?
            .map_ref(AppModel::populate_samples_listmodel)),

        AppMessage::LoadFromSavefile(filename) => {
            log::log!(log::Level::Info, "Loading from {filename}");

            match Savefile::load(&filename) {
                Ok(loaded_app_model) => {
                    loaded_app_model.load_enabled_sources()?;
                    loaded_app_model.populate_samples_listmodel();

                    Ok(AppModel {
                        viewvalues: ViewValues {
                            samples_listview_model: loaded_app_model
                                .viewvalues
                                .samples_listview_model,
                            ..model.viewvalues
                        },
                        sources_order: loaded_app_model.sources_order,
                        sources: loaded_app_model.sources,
                        samples: loaded_app_model.samples,
                        ..model
                    })
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

        AppMessage::DialogError(error) => {
            match error.kind::<DialogError>() {
                Some(e) => match e {
                    DialogError::Failed => log::log!(log::Level::Error, "Dialog failed: {e:?}"),
                    DialogError::Cancelled => (),
                    DialogError::Dismissed => (),
                    _ => log::log!(log::Level::Error, "Dialog error: {e:?}"),
                },
                None => log::log!(log::Level::Error, "Unknown dialog error: {error:?}"),
            };

            Ok(model)
        }
    }
}

fn update_view(model_ptr: AppModelPtr, old: AppModel, new: AppModel, view: &AsampoView) {
    macro_rules! maybe_update_text {
        ($old:ident, $new:ident, $view:ident, $entry:ident) => {
            if $old.viewvalues.$entry != $new.viewvalues.$entry
                && $view.$entry.text() != $new.viewvalues.$entry
            {
                $view.$entry.set_text(&$new.viewvalues.$entry);
            }
        };
    }

    maybe_update_text!(old, new, view, settings_latency_approx_label);

    maybe_update_text!(old, new, view, sources_add_fs_name_entry);
    maybe_update_text!(old, new, view, sources_add_fs_path_entry);
    maybe_update_text!(old, new, view, sources_add_fs_extensions_entry);

    if new.viewflags.sources_add_fs_browse {
        view::dialogs::choose_folder(
            model_ptr.clone(),
            view,
            AppMessage::AddFilesystemSourcePathBrowseSubmitted,
            AppMessage::AddFilesystemSourcePathBrowseError,
        );
    }

    if old.viewflags.sources_add_fs_fields_valid != new.viewflags.sources_add_fs_fields_valid {
        view.sources_add_fs_add_button
            .set_sensitive(new.viewflags.sources_add_fs_fields_valid);
    }

    if old.sources != new.sources {
        update_sources_list(model_ptr, new.clone(), view);
    }

    if old.viewvalues.samples_listview_model != new.viewvalues.samples_listview_model {
        view.samples_listview
            .set_model(Some(&gtk::SingleSelection::new(Some(
                new.viewvalues.samples_listview_model.clone(),
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
        // init css
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_resource("/style.css");

        gtk::style_context_add_provider_for_display(
            &Display::default().expect("There should be an available display"),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // init config
        let config = match ConfigFile::load(&ConfigFile::default_path()) {
            Ok(loaded_config) => {
                log::log!(
                    log::Level::Info,
                    "Loaded config: {}",
                    loaded_config.config_save_path
                );
                loaded_config
            }
            Err(e) => {
                log::log!(log::Level::Error, "Error loading config: {e:?}");
                log::log!(log::Level::Info, "Using default config");
                AppConfig::default()
            }
        };

        ConfigFile::save(&config, &ConfigFile::default_path()).unwrap();

        // init audio
        let (tx, rx) = mpsc::channel();
        let audiothread_handle = Rc::new(audiothread::spawn(
            rx,
            Some(
                audiothread::Opts::default()
                    .with_name("Asampo")
                    .with_sr_conv_quality(config.sample_rate_conversion_quality.clone())
                    .with_bufsize_n_stereo_samples(config.buffer_size_samples.into()),
            ),
        ));

        let view = AsampoView::new(app);

        let model = AppModel::new(
            Some(config),
            None,
            Some(tx.clone()),
            Some(audiothread_handle.clone()),
        );
        let model_ptr = Rc::new(Cell::new(Some(model.clone())));

        setup_settings_page(model_ptr.clone(), &view);
        setup_sources_page(model_ptr.clone(), &view);
        setup_samples_page(model_ptr.clone(), &view);

        build_actions(app, model_ptr, &view);

        view.present();
    });

    app.run()
}
