// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

mod config;
mod configfile;

#[macro_use]
mod ext;

mod model;
mod savefile;
mod testutils;
mod util;
mod view;

use std::{cell::Cell, io::BufReader, rc::Rc, sync::mpsc, thread, time::Duration};

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
    samplesets::{BaseSampleSet, SampleSet},
    sources::{file_system_source::FilesystemSource, Source},
};

use config::OptionMapExt;
use ext::WithModel;
use model::{AppModel, AppModelPtr, ViewFlags, ViewValues};

#[cfg(not(test))]
use savefile::Savefile;

#[cfg(test)]
use testutils::savefile_for_test::Savefile;

use view::{
    dialogs,
    menus::build_actions,
    samples::{setup_samples_page, SampleListEntry},
    sets::setup_sets_page,
    settings::setup_settings_page,
    sources::{setup_sources_page, update_sources_list},
    AsampoView,
};

use crate::view::{samples::update_samples_sidebar, sets::update_samplesets_list};

#[derive(Debug)]
enum ErrorWithEffect {
    AlertDialog { text: String, detail: String },
}

impl std::fmt::Display for ErrorWithEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorWithEffect::AlertDialog { text, detail } => {
                f.write_str(&format!("{}: {}", text, detail))
            }
        }
    }
}

impl std::error::Error for ErrorWithEffect {}

#[derive(Debug, Clone)]
enum InputDialogContext {
    AddToSampleset,
}

#[derive(Debug, Clone)]
enum SelectFolderDialogContext {
    BrowseForFilesystemSource,
}

#[derive(Debug)]
enum AppMessage {
    TimerTick,
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
    SampleSidebarAddToSetClicked,
    SampleSidebarAddToMostRecentlyUsedSetClicked,
    SourceEnabled(Uuid),
    SourceDisabled(Uuid),
    SourceDeleteClicked(Uuid),
    LoadFromSavefile(String),
    SaveToSavefile(String),
    DialogError(gtk::glib::Error),
    AddSampleSetNameChanged(String),
    AddSampleSetClicked,
    InputDialogOpened,
    InputDialogSubmitted(InputDialogContext, String),
    InputDialogCanceled(InputDialogContext),
    SelectFolderDialogOpened(SelectFolderDialogContext),
}

fn update(model_ptr: AppModelPtr, view: &AsampoView, message: AppMessage) {
    match message {
        AppMessage::TimerTick => (),
        _ => log::log!(log::Level::Debug, "{message:?}"),
    }

    let old_model = model_ptr.take().unwrap();

    match update_model(old_model.clone(), message) {
        Ok(new_model) => {
            model_ptr.set(Some(new_model.clone()));
            update_view(model_ptr, old_model, new_model, view);
        }

        Err(e) => {
            model_ptr.set(Some(old_model));
            log::log!(log::Level::Error, "{}", e.to_string());

            if e.is::<ErrorWithEffect>() {
                let e = e.downcast::<ErrorWithEffect>().unwrap();

                match e {
                    ErrorWithEffect::AlertDialog { text, detail } => {
                        dialogs::alert(model_ptr.clone(), view, &text, &detail)
                    }
                }
            }
        }
    }
}

fn get_or_create_sampleset(model: AppModel, name: &str) -> Result<(AppModel, Uuid), anyhow::Error> {
    match model
        .samplesets
        .iter()
        .find(|(_, set)| set.name() == name)
        .map(|(uuid, _)| *uuid)
    {
        Some(uuid) => Ok((model, uuid)),
        None => {
            let new_set = SampleSet::BaseSampleSet(BaseSampleSet::new(&name));
            let new_uuid = new_set.uuid().clone();

            Ok((model.add_sampleset(new_set), new_uuid))
        }
    }
}

fn add_selected_sample_to_sampleset_by_uuid(
    model: AppModel,
    uuid: &Uuid,
) -> Result<AppModel, anyhow::Error> {
    let sample = model
        .viewvalues
        .samples_selected_sample
        .as_ref()
        .ok_or(anyhow!("No selected sample"))?;

    let source = model
        .sources
        .get(
            sample
                .source_uuid()
                .ok_or(anyhow!("Selected sample has no source"))?,
        )
        .ok_or(anyhow!("Could not obtain source for selected sample"))?;

    let mut model = model.clone();

    model
        .samplesets
        .get_mut(uuid)
        .ok_or(anyhow!("Sample set not found (by uuid)"))?
        .add(source, sample)?;

    Ok(AppModel {
        viewflags: ViewFlags {
            samples_sidebar_add_to_prev_enabled: true,
            ..model.viewflags
        },
        viewvalues: ViewValues {
            samples_set_most_recently_used: Some(*uuid),
            ..model.viewvalues
        },
        ..model
    })
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
        AppMessage::TimerTick => {
            if let Some(0) = model.config_save_timeout {
                let config = model
                    .config
                    .as_ref()
                    .expect("There should be an active config");

                log::log!(
                    log::Level::Info,
                    "Saving config to {:?}",
                    config.config_save_path
                );
                ConfigFile::save(config, &config.config_save_path)?;

                log::log!(log::Level::Info, "Respawning audiothread with new config");

                if let Some(prev_tx) = model.audiothread_tx {
                    match prev_tx.send(audiothread::Message::Shutdown()) {
                        Ok(_) => thread::sleep(Duration::from_millis(10)),
                        Err(e) => {
                            log::log!(log::Level::Error, "Error shutting down audiothread: {e:?}")
                        }
                    }
                }

                let (tx, rx) = mpsc::channel();

                Ok(AppModel {
                    config_save_timeout: None,
                    audiothread_tx: Some(tx),
                    _audiothread_handle: Some(Rc::new(audiothread::spawn(
                        rx,
                        Some(
                            audiothread::Opts::default()
                                .with_name("asampo")
                                .with_sample_rate(config.output_samplerate_hz)
                                .with_sr_conv_quality(config.sample_rate_conversion_quality.clone())
                                .with_bufsize_n_stereo_samples(config.buffer_size_samples.into()),
                        ),
                    ))),
                    ..model
                })
            } else {
                Ok(AppModel {
                    config_save_timeout: model.config_save_timeout.map(|n| n - 1),
                    ..model
                })
            }
        }

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
                config_save_timeout: Some(3),
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
                config_save_timeout: Some(3),
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
            config_save_timeout: Some(3),
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
            config_save_timeout: Some(3),
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

            Ok(model)
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
            let model = model.add_source(new_source).enable_source(&uuid).unwrap();

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

                    Ok(AppModel {
                        viewvalues: ViewValues {
                            samples_selected_sample: Some(sample.borrow().clone()),
                            ..model.viewvalues
                        },
                        ..model
                    })
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

        AppMessage::SampleSidebarAddToSetClicked => Ok(AppModel {
            viewflags: ViewFlags {
                samples_sidebar_add_to_set_show_dialog: true,
                ..model.viewflags
            },
            ..model
        }),

        AppMessage::SampleSidebarAddToMostRecentlyUsedSetClicked => {
            let mru_uuid = model
                .viewvalues
                .samples_set_most_recently_used
                .ok_or(anyhow!("No sample set recently added to"))?;

            add_selected_sample_to_sampleset_by_uuid(model, &mru_uuid)
        }

        AppMessage::SourceEnabled(uuid) => Ok(model
            .enable_source(&uuid)?
            .map_ref(AppModel::populate_samples_listmodel)),

        AppMessage::SourceDisabled(uuid) => Ok(model
            .disable_source(&uuid)?
            .map_ref(AppModel::populate_samples_listmodel)),

        AppMessage::SourceDeleteClicked(uuid) => Ok(model
            .remove_source(&uuid)?
            .map_ref(AppModel::populate_samples_listmodel)),

        AppMessage::LoadFromSavefile(filename) => {
            log::log!(log::Level::Info, "Loading from {filename}");

            match Savefile::load(&filename) {
                Ok(loaded_app_model) => {
                    let model = AppModel {
                        sources: loaded_app_model.sources,
                        sources_order: loaded_app_model.sources_order,
                        samplesets: loaded_app_model.samplesets,
                        samplesets_order: loaded_app_model.samplesets_order,
                        ..model
                    };

                    model.samples.borrow_mut().clear();
                    model.load_enabled_sources()?;
                    model.populate_samples_listmodel();

                    Ok(model)
                }
                Err(e) => Err(anyhow::Error::new(ErrorWithEffect::AlertDialog {
                    text: "Error loading savefile".to_string(),
                    detail: e.to_string(),
                })),
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

        AppMessage::AddSampleSetNameChanged(text) => Ok(check_sources_add_fs_valid(AppModel {
            viewflags: ViewFlags {
                samplesets_add_fields_valid: !text.is_empty(),
                ..model.viewflags
            },
            viewvalues: ViewValues {
                samplesets_add_name_entry: text,
                ..model.viewvalues
            },
            ..model
        })),

        AppMessage::AddSampleSetClicked => {
            assert!(!model.viewvalues.samplesets_add_name_entry.is_empty());

            let set = SampleSet::BaseSampleSet(BaseSampleSet::new(
                &model.viewvalues.samplesets_add_name_entry,
            ));

            let result = model.add_sampleset(set);

            Ok(AppModel {
                viewflags: ViewFlags {
                    samplesets_add_fields_valid: false,
                    ..result.viewflags
                },
                viewvalues: ViewValues {
                    samplesets_add_name_entry: "".to_string(),
                    ..result.viewvalues
                },
                ..result
            })
        }

        AppMessage::InputDialogOpened => Ok(AppModel {
            viewflags: ViewFlags {
                samples_sidebar_add_to_set_show_dialog: false,
                ..model.viewflags
            },
            ..model
        }),

        AppMessage::InputDialogCanceled(_context) => Ok(model),

        AppMessage::InputDialogSubmitted(context, text) => match context {
            InputDialogContext::AddToSampleset => {
                let (model, set_uuid) = get_or_create_sampleset(model, &text)?;
                add_selected_sample_to_sampleset_by_uuid(model, &set_uuid)
            }
        },

        AppMessage::SelectFolderDialogOpened(context) => match context {
            SelectFolderDialogContext::BrowseForFilesystemSource => Ok(AppModel {
                viewflags: ViewFlags {
                    sources_add_fs_browse: false,
                    ..model.viewflags
                },
                ..model
            }),
        },
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
    maybe_update_text!(old, new, view, samplesets_add_name_entry);

    if new.viewflags.sources_add_fs_browse {
        dialogs::choose_folder(
            model_ptr.clone(),
            view,
            SelectFolderDialogContext::BrowseForFilesystemSource,
            AppMessage::AddFilesystemSourcePathBrowseSubmitted,
            AppMessage::AddFilesystemSourcePathBrowseError,
        );
    }

    if new.viewflags.samples_sidebar_add_to_set_show_dialog {
        dialogs::input(
            model_ptr.clone(),
            view,
            InputDialogContext::AddToSampleset,
            "Add to set",
            "Name of set:",
            "Favorites",
            "Add",
        );
    }

    if old.viewflags.sources_add_fs_fields_valid != new.viewflags.sources_add_fs_fields_valid {
        view.sources_add_fs_add_button
            .set_sensitive(new.viewflags.sources_add_fs_fields_valid);
    }

    if old.sources != new.sources {
        update_sources_list(model_ptr.clone(), new.clone(), view);
    }

    if old.viewvalues.samples_selected_sample != new.viewvalues.samples_selected_sample {
        update_samples_sidebar(model_ptr.clone(), new.clone(), view);
    }

    if old.viewflags.samples_sidebar_add_to_prev_enabled
        != new.viewflags.samples_sidebar_add_to_prev_enabled
    {
        view.samples_sidebar_add_to_prev_button
            .set_sensitive(new.viewflags.samples_sidebar_add_to_prev_enabled);
    }

    if old.viewvalues.samples_set_most_recently_used
        != new.viewvalues.samples_set_most_recently_used
    {
        if let Some(ref mru) = new.viewvalues.samples_set_most_recently_used {
            if let Some((_, set)) = new.samplesets.iter().find(|(uuid, _set)| *uuid == mru) {
                view.samples_sidebar_add_to_prev_button
                    .set_label(&format!("Add to '{}'", set.name()));
            }
        }
    }

    if old.viewflags.samplesets_add_fields_valid != new.viewflags.samplesets_add_fields_valid {
        view.samplesets_add_add_button
            .set_sensitive(new.viewflags.samplesets_add_fields_valid);
    }

    if old.samplesets != new.samplesets {
        update_samplesets_list(model_ptr.clone(), new.clone(), view);

        if new.viewvalues.samples_selected_sample.is_some() {
            update_samples_sidebar(model_ptr, new, view);
        }
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
                    .with_name("asampo")
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
        setup_sets_page(model_ptr.clone(), &view);

        build_actions(app, model_ptr.clone(), &view);

        view.present();

        gtk::glib::timeout_add_seconds_local(
            1,
            clone!(@strong model_ptr, @strong view => move || {
                update(model_ptr.clone(), &view, AppMessage::TimerTick);
                gtk::glib::ControlFlow::Continue
            }),
        );
    });

    app.run()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use libasampo::{
        samples::{BaseSample, Sample, SampleURI},
        sources::FakeSource,
    };

    use super::*;
    use crate::testutils::savefile_for_test;

    fn make_fake_source(name: &str, uri: &str, samples: &[&str]) -> (Uuid, FakeSource) {
        let uuid = Uuid::new_v4();

        let source = FakeSource {
            name: Some(name.to_string()),
            uri: uri.to_string(),
            uuid,
            list: samples
                .iter()
                .map(|s| {
                    Sample::BaseSample(BaseSample::new(
                        &SampleURI(s.to_string()),
                        s,
                        &libasampo::samples::SampleMetadata {
                            rate: 48000,
                            channels: 2,
                            src_fmt_display: "PCM".to_string(),
                            size_bytes: Some(0),
                            length_millis: Some(0),
                        },
                        Some(uuid),
                    ))
                })
                .collect(),
            list_error: None,
            stream: HashMap::new(),
            stream_error: None,
            enabled: true,
        };

        (uuid, source)
    }

    #[test]
    fn test_bug_loading_savefile_samples_not_assigned() {
        savefile_for_test::LOAD.set(Some(|_| -> Result<AppModel, anyhow::Error> {
            let (_, source) = make_fake_source("", "", &["first.wav"]);

            Ok(AppModel::new(Some(AppConfig::default()), None, None, None)
                .add_source(Source::FakeSource(source)))
        }));

        let model = AppModel::new(Some(AppConfig::default()), None, None, None);
        let model = update_model(model, AppMessage::LoadFromSavefile("".to_string())).unwrap();

        let (uuid, source) = make_fake_source("", "", &["second.wav"]);
        let model = model
            .add_source(Source::FakeSource(source))
            .enable_source(&uuid)
            .unwrap();

        assert_eq!(model.samples.borrow().len(), 2);
    }

    #[test]
    fn test_using_real_savefile_in_test() {
        use libasampo::sources::{file_system_source::FilesystemSource, Source};

        savefile_for_test::LOAD.set(Some(savefile::Savefile::load));
        savefile_for_test::SAVE.set(Some(savefile::Savefile::save));

        let tmpfile = tempfile::NamedTempFile::new()
            .expect("Should be able to create temporary file")
            .into_temp_path();

        let src = Source::FilesystemSource(FilesystemSource::new_named(
            "abc123".to_string(),
            "/tmp".to_string(),
            ["mp3".to_string()].to_vec(),
        ));

        let uuid = *src.uuid();

        Savefile::save(
            &AppModel::new(Some(AppConfig::default()), None, None, None).add_source(src),
            tmpfile
                .to_str()
                .expect("Temporary file should have UTF-8 filename"),
        )
        .expect("Should be able to Savefile::save to a temporary file");

        let model = Savefile::load(
            tmpfile
                .to_str()
                .expect("Temporary file should have UTF-8 filename"),
        )
        .expect("Should be able to Savefile::load from temporary file");

        assert_eq!(
            model
                .sources
                .get(&uuid)
                .expect("Loaded model should contain the fake source")
                .name(),
            Some("abc123")
        );
    }
}
