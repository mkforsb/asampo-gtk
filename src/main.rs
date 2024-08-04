// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{cell::Cell, rc::Rc, sync::mpsc};

use audiothread::{AudioSpec, NonZeroNumFrames};

use gtk::{
    gdk::Display,
    gio::ApplicationFlags,
    glib::{clone, ExitCode},
    prelude::*,
    Application,
};

mod appmessage;
mod config;
mod configfile;
mod ext;
mod labels;
mod model;
mod savefile;
mod testutils;
mod timers;
mod update_model;
mod update_view;
mod util;
mod view;

use crate::{
    appmessage::AppMessage,
    config::AppConfig,
    configfile::ConfigFile,
    ext::WithModel,
    model::{AppModel, AppModelPtr},
    update_model::update_model,
    update_view::update_view,
    view::{
        dialogs, menus::build_actions, samples::setup_samples_page,
        sequences::setup_sequences_page, sets::setup_sets_page, settings::setup_settings_page,
        sources::setup_sources_page, AsampoView,
    },
};

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

fn update(model_ptr: AppModelPtr, view: &AsampoView, message: AppMessage) {
    match message {
        AppMessage::TimerTick => (),
        AppMessage::SourceLoadingMessage(..) => (),
        AppMessage::DrumMachinePlaybackEvent(..) => (),
        _ => log::log!(log::Level::Debug, "{message:?}"),
    }

    if let AppMessage::Sequence(messages) = message {
        for message in messages {
            update(model_ptr.clone(), view, message);
        }

        return;
    }

    let old_model = model_ptr.take().unwrap();

    match update_model(old_model.clone(), message) {
        Ok(new_model) => {
            model_ptr.set(Some(new_model.clone()));
            update_view(model_ptr.clone(), old_model, new_model.clone(), view);
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

fn main() -> ExitCode {
    env_logger::init();

    gtk::gio::resources_register_include!("resources.gresource")
        .expect("Should be able to register compiled GTK resources.");

    let app = Application::builder()
        .application_id("io.github.mkforsb.asampo_gtk")
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
        let _ = Rc::new(audiothread::spawn(
            rx,
            Some(
                audiothread::Opts::default()
                    .with_name("asampo")
                    .with_spec(
                        AudioSpec::new(config.output_samplerate_hz, 2).unwrap_or_else(|_| {
                            log::log!(
                                log::Level::Warn,
                                "Invalid sample rate in config, using default"
                            );
                            AudioSpec::new(48000, 2).unwrap()
                        }),
                    )
                    .with_conversion_quality(config.sample_rate_conversion_quality)
                    .with_buffer_size(
                        (config.buffer_size_frames as usize)
                            .try_into()
                            .unwrap_or_else(|_| {
                                log::log!(
                                    log::Level::Warn,
                                    "Invalid buffer size in config, using default"
                                );
                                NonZeroNumFrames::new(1024).unwrap()
                            }),
                    ),
            ),
        ));

        let view = AsampoView::new(app);

        let model = AppModel::new(config, None, tx.clone());
        let model_ptr = Rc::new(Cell::new(Some(model.clone())));

        setup_settings_page(model_ptr.clone(), &view);
        setup_sources_page(model_ptr.clone(), &view);
        setup_samples_page(model_ptr.clone(), &view);
        setup_sets_page(model_ptr.clone(), &view);
        setup_sequences_page(model_ptr.clone(), &view);

        build_actions(app, model_ptr.clone(), &view);

        view.titlebar_stop_button.connect_clicked(
            clone!(@strong model_ptr, @strong view => move |_| {
                update(model_ptr.clone(), &view, AppMessage::StopAllSoundButtonClicked);
            }),
        );

        view.connect_close_request(clone!(@strong model_ptr, @strong view => move |_| {
            update(model_ptr.clone(), &view, AppMessage::QuitRequested);
            gtk::glib::Propagation::Stop
        }));

        view.present();

        timers::init_timertick_timer(model_ptr.clone(), &view);
        timers::init_messaging_timer(model_ptr.clone(), &view);
        timers::init_drum_machine_events_timer(model_ptr.clone(), &view);
    });

    app.run()
}

#[cfg(test)]
mod tests {
    use libasampo::sources::SourceOps;
    use savefile::Savefile;

    use super::*;
    use crate::testutils::savefile_for_test;

    #[test]
    fn test_using_real_savefile_in_test() {
        use libasampo::sources::{file_system_source::FilesystemSource, Source};

        savefile_for_test::LOAD.set(Some(|path| match savefile::Savefile::load(path) {
            Ok(loaded_savefile) => Ok(savefile_for_test::Savefile {
                sources_domained: loaded_savefile.sources_domained()?,
                sets_domained: loaded_savefile.sets_domained()?,
                sequences_domained: loaded_savefile.sequences_domained()?,
            }),
            Err(e) => Err(e),
        }));

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

        let (dummy_tx, _) = mpsc::channel::<audiothread::Message>();

        Savefile::save(
            &AppModel::new(AppConfig::default(), None, dummy_tx)
                .add_source(src)
                .unwrap(),
            tmpfile
                .to_str()
                .expect("Temporary file should have UTF-8 filename"),
        )
        .expect("Should be able to Savefile::save to a temporary file");

        let loaded_savefile = Savefile::load(
            tmpfile
                .to_str()
                .expect("Temporary file should have UTF-8 filename"),
        )
        .expect("Should be able to Savefile::load from temporary file");

        assert_eq!(
            loaded_savefile
                .sources_domained()
                .unwrap()
                .iter()
                .find(|s| *s.uuid() == uuid)
                .expect("Loaded model should contain the fake source")
                .name(),
            Some("abc123")
        );
    }
}
