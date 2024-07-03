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

use std::{
    cell::Cell,
    io::BufReader,
    path::Path,
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use audiothread::{AudioSpec, NonZeroNumFrames};
use ext::ClonedHashMapExt;
use model::{DrumMachineModel, ExportState, ModelOps};
use uuid::Uuid;

use gtk::{
    gdk::Display,
    gio::ApplicationFlags,
    glib::{clone, ExitCode},
    prelude::*,
    Application, DialogError,
};

use libasampo::{
    prelude::*,
    samples::Sample,
    samplesets::{
        export::{Conversion, ExportJob, ExportJobMessage},
        BaseSampleSet, DrumkitLabelling, SampleSet, SampleSetLabelling,
    },
    sequences::{drumkit_render_thread, DrumkitSequenceEvent},
};

use crate::{
    config::AppConfig,
    configfile::ConfigFile,
    ext::WithModel,
    model::{AppModel, AppModelOps, AppModelPtr, ViewFlags, ViewModelOps, ViewValues},
    util::gtk_find_child_by_builder_id,
    view::{
        dialogs,
        menus::build_actions,
        samples::{setup_samples_page, update_samples_sidebar, SampleListEntry},
        sequences::{
            setup_sequences_page, update_drum_machine_view, LABELS as DRUM_MACHINE_VIEW_LABELS,
        },
        sets::{setup_sets_page, update_samplesets_detail, update_samplesets_list, LabellingKind},
        settings::setup_settings_page,
        sources::{setup_sources_page, update_sources_list},
        AsampoView,
    },
};

#[cfg(not(test))]
use crate::savefile::Savefile;

#[cfg(test)]
use crate::testutils::savefile_for_test::Savefile;

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
    CreateSampleSet,
}

#[derive(Debug, Clone)]
enum SelectFolderDialogContext {
    BrowseForFilesystemSource,
    BrowseForExportTargetDirectory,
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
    SampleListSampleSelected(u32),
    SampleSetSampleSelected(Sample),
    SamplesFilterChanged(String),
    SampleSidebarAddToSetClicked,
    SampleSidebarAddToMostRecentlyUsedSetClicked,
    SourceEnabled(Uuid),
    SourceDisabled(Uuid),
    SourceDeleteClicked(Uuid),
    SourceLoadingMessage(Uuid, Vec<Result<Sample, libasampo::errors::Error>>),
    SourceLoadingDisconnected(Uuid),
    LoadFromSavefile(String),
    SaveToSavefile(String),
    DialogError(gtk::glib::Error),
    AddSampleSetClicked,
    InputDialogOpened(InputDialogContext),
    InputDialogSubmitted(InputDialogContext, String),
    InputDialogCanceled(InputDialogContext),
    SelectFolderDialogOpened(SelectFolderDialogContext),
    SampleSetSelected(Uuid),
    SampleSetLabellingKindChanged(LabellingKind),
    SampleSetDetailsExportClicked,
    ExportDialogOpened(dialogs::ExportDialogView),
    ExportDialogClosed,
    ExportTargetDirectoryChanged(String),
    ExportTargetDirectoryBrowseClicked,
    ExportTargetDirectoryBrowseSubmitted(String),
    ExportTargetDirectoryBrowseError(gtk::glib::Error),
    PerformExportClicked,
    PlainCopyExportSelected,
    ConversionExportSelected,
    ExportJobMessage(libasampo::samplesets::export::ExportJobMessage),
    ExportJobDisconnected,
    StopAllSoundButtonClicked,
    DrumMachineTempoChanged(u16),
    DrumMachineSwingChanged(u32),
    DrumMachinePlayClicked,
    DrumMachineStopClicked,
    DrumMachineBackClicked,
    DrumMachineSaveSequenceClicked,
    DrumMachineSaveSequenceAsClicked,
    DrumMachineSaveSampleSetClicked,
    DrumMachineSaveSampleSetAsClicked,
    DrumMachinePadClicked(usize),
    DrumMachinePartClicked(usize),
    DrumMachineStepClicked(usize),
    DrumMachinePlaybackEvent(DrumkitSequenceEvent),
}

fn update(model_ptr: AppModelPtr, view: &AsampoView, message: AppMessage) {
    match message {
        AppMessage::TimerTick => (),
        AppMessage::SourceLoadingMessage(..) => (),
        AppMessage::DrumMachinePlaybackEvent(..) => (),
        _ => log::log!(log::Level::Debug, "{message:?}"),
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

fn update_model(model: AppModel, message: AppMessage) -> Result<AppModel, anyhow::Error> {
    match message {
        AppMessage::TimerTick => {
            if !model.sources_loading.is_empty() {
                model.populate_samples_listmodel();
            }

            if model
                .config_save_timeout
                .is_some_and(|t| t <= Instant::now())
            {
                let config = &model.config;

                log::log!(
                    log::Level::Info,
                    "Saving config to {:?}",
                    config.config_save_path
                );
                ConfigFile::save(config, &config.config_save_path)?;

                log::log!(log::Level::Info, "Respawning audiothread with new config");

                let had_dks_render_thread = model.drum_machine.render_thread_tx.is_some();

                if let Some(control_tx) = &model.drum_machine.render_thread_tx {
                    match control_tx.send(drumkit_render_thread::Message::Shutdown) {
                        Ok(_) => (),
                        Err(e) => {
                            log::log!(
                                log::Level::Error,
                                "Error shutting down drumkit sequence render thread: {e}"
                            );
                        }
                    }
                }

                // TODO: sequence the shutdown in some other way to avoid sleeping the main thread
                // give drum machine thread some time to shut down gracefully
                std::thread::sleep(Duration::from_millis(250));

                if let Some(prev_tx) = model.audiothread_tx {
                    match prev_tx.send(audiothread::Message::Shutdown) {
                        Ok(_) => {
                            // give audiothread some time to shut down gracefully
                            std::thread::sleep(Duration::from_millis(10))
                        }
                        Err(e) => {
                            log::log!(log::Level::Error, "Error shutting down audiothread: {e}")
                        }
                    }
                }

                let (audiothread_tx, audiothread_rx) = mpsc::channel::<audiothread::Message>();

                let _audiothread_handle = Some(Rc::new(audiothread::spawn(
                    audiothread_rx,
                    Some(
                        audiothread::Opts::default()
                            .with_name("asampo")
                            .with_spec(AudioSpec::new(config.output_samplerate_hz, 2)?)
                            .with_conversion_quality(config.sample_rate_conversion_quality)
                            .with_buffer_size((config.buffer_size_frames as usize).try_into()?),
                    ),
                )));

                let drum_machine = if had_dks_render_thread {
                    DrumMachineModel::new_with_render_thread(audiothread_tx.clone())
                } else {
                    DrumMachineModel::new(None, None)
                };

                Ok(AppModel {
                    audiothread_tx: Some(audiothread_tx.clone()),
                    _audiothread_handle,
                    drum_machine,
                    ..model
                }
                .clear_config_save_timeout())
            } else {
                Ok(model)
            }
        }

        AppMessage::SettingsOutputSampleRateChanged(choice) => {
            let new_config = model.config.clone().with_samplerate_choice(choice);

            Ok(model
                .set_latency_approx_label_by_config(&new_config)
                .set_config(new_config)
                .set_config_save_timeout(Instant::now() + Duration::from_secs(3)))
        }

        AppMessage::SettingsBufferSizeChanged(samples) => {
            let new_config = model.config.clone().with_buffer_size(samples);

            Ok(model
                .set_latency_approx_label_by_config(&new_config)
                .set_config(new_config)
                .set_config_save_timeout(Instant::now() + Duration::from_secs(3)))
        }

        AppMessage::SettingsSampleRateConversionQualityChanged(choice) => {
            let new_config = model.config.clone().with_conversion_quality_choice(choice);

            Ok(model
                .set_config(new_config)
                .set_config_save_timeout(Instant::now() + Duration::from_secs(3)))
        }

        AppMessage::SettingsSamplePlaybackBehaviorChanged(choice) => {
            let new_config = model
                .config
                .clone()
                .with_sample_playback_behavior_choice(choice);

            Ok(model
                .set_config(new_config)
                .set_config_save_timeout(Instant::now() + Duration::from_secs(3)))
        }

        AppMessage::AddFilesystemSourceNameChanged(text) => Ok(model
            .set_sources_add_fs_name_entry(text)
            .validate_sources_add_fs_fields()),

        AppMessage::AddFilesystemSourcePathChanged(text) => Ok(model
            .set_sources_add_fs_path_entry(text)
            .validate_sources_add_fs_fields()),

        AppMessage::AddFilesystemSourcePathBrowseClicked => {
            Ok(model.signal_sources_add_fs_begin_browse())
        }

        AppMessage::AddFilesystemSourcePathBrowseSubmitted(text) => Ok(match (
            model.viewvalues.sources_add_fs_name_entry.is_empty(),
            Path::new(&text).file_name(),
        ) {
            (true, Some(filename)) => model
                .set_sources_add_fs_name_entry(
                    filename
                        .to_str()
                        .ok_or(anyhow!("Path contains invalid UTF-8"))?,
                )
                .set_sources_add_fs_path_entry(text),

            _ => model.set_sources_add_fs_path_entry(text),
        }
        .validate_sources_add_fs_fields()),

        AppMessage::AddFilesystemSourcePathBrowseError(error) => {
            log::log!(log::Level::Debug, "Error browsing for folder: {error:?}");

            Ok(model)
        }

        AppMessage::AddFilesystemSourceExtensionsChanged(text) => Ok(model
            .set_sources_add_fs_extensions_entry(text)
            .validate_sources_add_fs_fields()),

        AppMessage::AddFilesystemSourceClicked => Ok(model
            .commit_file_system_source()?
            .tap(AppModel::populate_samples_listmodel)),

        AppMessage::SourceLoadingMessage(uuid, messages) => {
            let mut samples = model.samples.borrow_mut();
            let len_before = samples.len();

            for message in messages {
                match message {
                    Ok(sample) => {
                        samples.push(sample);
                    }

                    Err(e) => log::log!(log::Level::Error, "Error loading source: {e}"),
                }
            }

            let added = samples.len() - len_before;
            drop(samples);

            model.source_sample_count_add(uuid, added)
        }

        AppMessage::SourceLoadingDisconnected(uuid) => {
            model.populate_samples_listmodel();

            Ok(AppModel {
                sources_loading: model.sources_loading.clone_and_remove(&uuid)?,
                ..model
            })
        }

        AppMessage::SampleListSampleSelected(index) => {
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

                    model
                        .audiothread_tx
                        .as_ref()
                        .unwrap()
                        .send(audiothread::Message::PlaySymphoniaSource(
                            audiothread::SymphoniaSource::from_buf_reader(BufReader::new(stream))?,
                        ))
                        .map_err(|_| anyhow!("Send error on audio thread control channel"))?;

                    Ok(AppModel {
                        samplelist_selected_sample: Some(sample.borrow().clone()),
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
        .tap(AppModel::populate_samples_listmodel)),

        AppMessage::SampleSidebarAddToSetClicked => Ok(AppModel {
            viewflags: ViewFlags {
                samples_sidebar_add_to_set_show_dialog: true,
                ..model.viewflags
            },
            ..model
        }),

        AppMessage::SampleSidebarAddToMostRecentlyUsedSetClicked => {
            let mru_uuid = model
                .sets_most_recently_used_uuid
                .ok_or(anyhow!("No sample set recently added to"))?;

            model::util::add_selected_sample_to_sampleset_by_uuid(model, &mru_uuid)
        }

        AppMessage::SourceEnabled(uuid) => {
            let source = model
                .sources
                .get(&uuid)
                .ok_or(anyhow!("Source not found"))?;

            let (tx, rx) = std::sync::mpsc::channel::<Result<Sample, libasampo::errors::Error>>();

            std::thread::spawn(clone!(@strong source => move || {
                source.list_async(tx);
            }));

            Ok(AppModel {
                sources_loading: model.sources_loading.clone_and_insert(uuid, Rc::new(rx)),
                ..model
            }
            .reset_source_sample_count(uuid)?
            .enable_source(&uuid)?
            .tap(AppModel::populate_samples_listmodel))
        }

        AppMessage::SourceDisabled(uuid) => Ok(model
            .disable_source(&uuid)?
            .tap(AppModel::populate_samples_listmodel)),

        AppMessage::SourceDeleteClicked(uuid) => Ok(model
            .remove_source(&uuid)?
            .tap(AppModel::populate_samples_listmodel)),

        AppMessage::LoadFromSavefile(filename) => {
            log::log!(log::Level::Info, "Loading from {filename}");

            match Savefile::load(&filename) {
                Ok(loaded_app_model) => {
                    let model = AppModel {
                        viewvalues: ViewValues {
                            sources_sample_count: loaded_app_model
                                .sources
                                .keys()
                                .map(|k| (*k, 0))
                                .collect(),
                            ..model.viewvalues
                        },
                        sources: loaded_app_model.sources,
                        sources_order: loaded_app_model.sources_order,
                        sets: loaded_app_model.sets,
                        sets_order: loaded_app_model.sets_order,
                        ..model
                    };

                    model.samples.borrow_mut().clear();
                    model.populate_samples_listmodel();

                    Ok(AppModel {
                        sources_loading: model
                            .sources
                            .iter()
                            .filter_map(|(uuid, source)| match source.is_enabled() {
                                true => {
                                    let (tx, rx) = std::sync::mpsc::channel::<
                                        Result<Sample, libasampo::errors::Error>,
                                    >();
                                    std::thread::spawn(clone!(@strong source => move || {
                                        source.list_async(tx);
                                    }));

                                    Some((*uuid, Rc::new(rx)))
                                }

                                false => None,
                            })
                            .collect(),
                        ..model
                    })
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

        AppMessage::AddSampleSetClicked => Ok(AppModel {
            viewflags: ViewFlags {
                sets_add_set_show_dialog: true,
                ..model.viewflags
            },
            ..model
        }),

        AppMessage::InputDialogOpened(context) => match context {
            InputDialogContext::AddToSampleset => Ok(AppModel {
                viewflags: ViewFlags {
                    samples_sidebar_add_to_set_show_dialog: false,
                    ..model.viewflags
                },
                ..model
            }),

            InputDialogContext::CreateSampleSet => Ok(AppModel {
                viewflags: ViewFlags {
                    sets_add_set_show_dialog: false,
                    ..model.viewflags
                },
                ..model
            }),
        },

        AppMessage::InputDialogCanceled(_context) => Ok(model),

        AppMessage::InputDialogSubmitted(context, text) => match context {
            InputDialogContext::AddToSampleset => {
                let (model, set_uuid) = model::util::get_or_create_sampleset(model, text)?;
                model::util::add_selected_sample_to_sampleset_by_uuid(model, &set_uuid)
            }

            InputDialogContext::CreateSampleSet => {
                Ok(model.add_sampleset(SampleSet::BaseSampleSet(BaseSampleSet::new(text))))
            }
        },

        // TODO: replace with function pointer, just like "ok" and "cancel" for input dialog?
        AppMessage::SelectFolderDialogOpened(context) => match context {
            SelectFolderDialogContext::BrowseForFilesystemSource => {
                Ok(model.clear_signal_sources_add_fs_begin_browse())
            }

            SelectFolderDialogContext::BrowseForExportTargetDirectory => Ok(AppModel {
                viewflags: ViewFlags {
                    sets_export_begin_browse: false,
                    ..model.viewflags
                },
                ..model
            }),
        },

        AppMessage::SampleSetSelected(uuid) => {
            let set = model
                .sets
                .get(&uuid)
                .ok_or(anyhow!("Sample set not found (by uuid)"))?;

            Ok(AppModel {
                viewflags: ViewFlags {
                    sets_export_enabled: set.len() > 0,
                    ..model.viewflags
                },
                sets_selected_set: Some(uuid),
                ..model
            })
        }

        AppMessage::SampleSetSampleSelected(sample) => {
            let stream = model
                .sources
                .get(
                    sample
                        .source_uuid()
                        .ok_or(anyhow!("Sample missing source uuid"))?,
                )
                .ok_or(anyhow!("Failed to get source for sample"))?
                .stream(&sample)?;

            model
                .audiothread_tx
                .as_ref()
                .unwrap()
                .send(audiothread::Message::PlaySymphoniaSource(
                    audiothread::SymphoniaSource::from_buf_reader(BufReader::new(stream))?,
                ))
                .map_err(|_| anyhow!("Send error on audio thread control channel"))?;

            Ok(model)
        }

        AppMessage::SampleSetLabellingKindChanged(kind) => {
            let set_uuid = model
                .sets_selected_set
                .ok_or(anyhow!("No sample set selected"))?;

            let mut result = model.clone();

            let set = result
                .sets
                .get_mut(&set_uuid)
                .ok_or(anyhow!("Sample set not found (by uuid)"))?;

            match set {
                SampleSet::BaseSampleSet(ref mut set) => match kind {
                    LabellingKind::None => set.set_labelling(None),
                    LabellingKind::Drumkit => set.set_labelling(Some(
                        SampleSetLabelling::DrumkitLabelling(DrumkitLabelling::new()),
                    )),
                },
            };

            Ok(result)
        }

        AppMessage::SampleSetDetailsExportClicked => Ok(AppModel {
            viewflags: ViewFlags {
                sets_export_show_dialog: true,
                ..model.viewflags
            },
            ..model
        }),

        AppMessage::ExportDialogOpened(dialogview) => Ok(AppModel {
            viewflags: ViewFlags {
                view_sensitive: false,
                sets_export_show_dialog: false,
                ..model.viewflags
            },
            viewvalues: ViewValues {
                sets_export_dialog_view: Some(dialogview),
                ..model.viewvalues
            },
            ..model
        }),

        AppMessage::ExportDialogClosed => Ok(AppModel {
            viewflags: ViewFlags {
                view_sensitive: true,
                ..model.viewflags
            },
            viewvalues: ViewValues {
                sets_export_dialog_view: None,
                ..model.viewvalues
            },
            ..model
        }),

        AppMessage::ExportTargetDirectoryChanged(text) => Ok(AppModel {
            viewflags: ViewFlags {
                sets_export_fields_valid: !text.is_empty(),
                ..model.viewflags
            },
            viewvalues: ViewValues {
                sets_export_target_dir_entry: text,
                ..model.viewvalues
            },
            ..model
        }),

        AppMessage::ExportTargetDirectoryBrowseClicked => Ok(AppModel {
            viewflags: ViewFlags {
                sets_export_begin_browse: true,
                ..model.viewflags
            },
            ..model
        }),

        AppMessage::ExportTargetDirectoryBrowseSubmitted(text) => Ok(AppModel {
            viewvalues: ViewValues {
                sets_export_target_dir_entry: text,
                ..model.viewvalues
            },
            ..model
        }),

        AppMessage::ExportTargetDirectoryBrowseError(_e) => Ok(model),

        AppMessage::PerformExportClicked => {
            use libasampo::samplesets::export::{RateConversionQuality, WavSampleFormat, WavSpec};

            let sampleset = model
                .sets
                .get(
                    &model
                        .sets_selected_set
                        .ok_or(anyhow!("No sample set selected"))?,
                )
                .ok_or(anyhow!("Broken state, sample set not found"))?
                .clone();

            let num_samples = sampleset.len();

            let (tx, rx) = std::sync::mpsc::channel::<ExportJobMessage>();

            std::thread::spawn(clone!(@strong model => move || {
                let job = ExportJob::new(
                    model
                        .viewvalues
                        .sets_export_target_dir_entry
                        .clone(),
                    match model.viewvalues.sets_export_kind {
                        None | Some(model::ExportKind::PlainCopy) => None,
                        Some(model::ExportKind::Conversion) => Some(Conversion::Wav(
                            WavSpec {
                                channels: 2,
                                sample_rate: 44100,
                                bits_per_sample: 16,
                                sample_format: WavSampleFormat::Int,
                            },
                            Some(RateConversionQuality::High),
                        )),
                    });

                job.perform(&sampleset, &model.sources, Some(tx));
            }));

            Ok(AppModel {
                sets_export_state: Some(model::ExportState::Exporting),
                sets_export_progress: Some((0, num_samples)),
                export_job_rx: Some(Rc::new(rx)),
                ..model
            })
        }

        AppMessage::PlainCopyExportSelected => Ok(AppModel {
            viewvalues: ViewValues {
                sets_export_kind: Some(model::ExportKind::PlainCopy),
                ..model.viewvalues
            },
            ..model
        }),

        AppMessage::ConversionExportSelected => Ok(AppModel {
            viewvalues: ViewValues {
                sets_export_kind: Some(model::ExportKind::Conversion),
                ..model.viewvalues
            },
            ..model
        }),

        AppMessage::ExportJobMessage(message) => match message {
            ExportJobMessage::ItemsCompleted(n) => Ok(AppModel {
                sets_export_progress: model.sets_export_progress.map(|(_, m)| (n, m)),
                ..model
            }),
            ExportJobMessage::Error(e) => Err(e.into()),
            ExportJobMessage::Finished => Ok(AppModel {
                sets_export_state: Some(ExportState::Finished),
                sets_export_progress: None,
                export_job_rx: None,
                ..model
            }),
        },

        AppMessage::ExportJobDisconnected => {
            log::log!(log::Level::Debug, "Export job disconnected");

            Ok(AppModel {
                export_job_rx: None,
                ..model
            })
        }

        AppMessage::StopAllSoundButtonClicked => {
            if let Some(dks_render_thread_tx) = &model.drum_machine.render_thread_tx {
                match dks_render_thread_tx.send(drumkit_render_thread::Message::Shutdown) {
                    Ok(_) => (),
                    Err(e) => log::log!(log::Level::Error, "Stop all sounds error: {e}"),
                }

                // TODO: find a good way to avoid having to sleep
                // give drum machine thread some time to shut down gracefully
                std::thread::sleep(std::time::Duration::from_millis(250));
            }

            if let Some(audiothread_tx) = &model.audiothread_tx {
                match audiothread_tx.send(audiothread::Message::DropAll) {
                    Ok(_) => (),
                    Err(e) => log::log!(log::Level::Error, "Stop all sounds error: {e}"),
                }
            }

            match &model.drum_machine.render_thread_tx {
                Some(_) => Ok(AppModel {
                    drum_machine: DrumMachineModel {
                        render_thread_tx: None,
                        ..model.drum_machine
                    },
                    ..model
                }),

                None => Ok(model),
            }
        }

        AppMessage::DrumMachineTempoChanged(tempo) => {
            if let Some(dks_render_thread_tx) = &model.drum_machine.render_thread_tx {
                let _ = dks_render_thread_tx
                    .send(drumkit_render_thread::Message::SetTempo(tempo.try_into()?));
            }

            Ok(model)
        }

        AppMessage::DrumMachineSwingChanged(swing) => {
            if let Some(dks_render_thread_tx) = &model.drum_machine.render_thread_tx {
                let _ = dks_render_thread_tx.send(drumkit_render_thread::Message::SetSwing(
                    (swing as f64 / 100.0).try_into()?,
                ));
            }

            Ok(model)
        }

        AppMessage::DrumMachinePlayClicked => Ok(model),
        AppMessage::DrumMachineStopClicked => Ok(model),
        AppMessage::DrumMachineBackClicked => Ok(model),
        AppMessage::DrumMachineSaveSequenceClicked => Ok(model),
        AppMessage::DrumMachineSaveSequenceAsClicked => Ok(model),
        AppMessage::DrumMachineSaveSampleSetClicked => Ok(model),
        AppMessage::DrumMachineSaveSampleSetAsClicked => Ok(model),
        AppMessage::DrumMachinePadClicked(n) => Ok(AppModel {
            drum_machine: DrumMachineModel {
                activated_pad: n,
                ..model.drum_machine
            },
            ..model
        }),
        AppMessage::DrumMachinePartClicked(_n) => Ok(model),
        AppMessage::DrumMachineStepClicked(n) => {
            let amp = 0.5f32;
            let mut new_sequence = model.drum_machine.sequence.clone();
            let label = DRUM_MACHINE_VIEW_LABELS[model.drum_machine.activated_pad];

            if new_sequence
                .labels_at_step(n)
                .ok_or(anyhow!("Drum machine sequence has no step {n}"))?
                .contains(&label)
            {
                new_sequence.unset_step_trigger(
                    n,
                    DRUM_MACHINE_VIEW_LABELS[model.drum_machine.activated_pad],
                );

                if let Some(render_thread_tx) = &model.drum_machine.render_thread_tx {
                    render_thread_tx
                        .send(
                            drumkit_render_thread::Message::EditSequenceUnsetStepTrigger {
                                step: n,
                                label,
                            },
                        )
                        .map_err(|e| {
                            anyhow!(
                                "Failed sending update event to drum sequence render thread: {e}"
                            )
                        })?;
                }
            } else {
                new_sequence.set_step_trigger(n, label, amp);

                if let Some(render_thread_tx) = &model.drum_machine.render_thread_tx {
                    render_thread_tx
                        .send(drumkit_render_thread::Message::EditSequenceSetStepTrigger {
                            step: n,
                            label,
                            amp,
                        })
                        .map_err(|e| {
                            anyhow!(
                                "Failed sending update event to drum sequence render thread: {e}"
                            )
                        })?;
                }
            }

            Ok(AppModel {
                drum_machine: DrumMachineModel {
                    sequence: new_sequence,
                    ..model.drum_machine
                },
                ..model
            })
        }

        AppMessage::DrumMachinePlaybackEvent(event) => Ok(AppModel {
            drum_machine: DrumMachineModel {
                event_latest: Some(event),
                ..model.drum_machine
            },
            ..model
        }),
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

        ($old:ident, $new:ident, expr $viewexpr: expr, $entry:ident) => {
            if $old.viewvalues.$entry != $new.viewvalues.$entry
                && ($viewexpr).text() != $new.viewvalues.$entry
            {
                ($viewexpr).set_text(&$new.viewvalues.$entry);
            }
        };
    }

    if old.viewflags.view_sensitive != new.viewflags.view_sensitive {
        view.set_sensitive(new.viewflags.view_sensitive);
    }

    maybe_update_text!(old, new, view, settings_latency_approx_label);
    maybe_update_text!(old, new, view, sources_add_fs_name_entry);
    maybe_update_text!(old, new, view, sources_add_fs_path_entry);
    maybe_update_text!(old, new, view, sources_add_fs_extensions_entry);

    if let Some(dialogview) = &new.viewvalues.sets_export_dialog_view {
        maybe_update_text!(
            old,
            new,
            expr dialogview.target_dir_entry,
            sets_export_target_dir_entry
        );

        if old.viewflags.sets_export_fields_valid != new.viewflags.sets_export_fields_valid {
            dialogview
                .export_button
                .set_sensitive(new.viewflags.sets_export_fields_valid);
        }
    }

    if new.viewflags.sources_add_fs_begin_browse {
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

    if new.viewflags.sets_add_set_show_dialog {
        dialogs::input(
            model_ptr.clone(),
            view,
            InputDialogContext::CreateSampleSet,
            "Create set",
            "Name of set:",
            "Favorites",
            "Create",
        );
    }

    if new.viewflags.sets_export_show_dialog {
        dialogs::sampleset_export(model_ptr.clone(), view, new.clone());
    }

    if new.viewflags.sets_export_begin_browse {
        dialogs::choose_folder(
            model_ptr.clone(),
            view,
            SelectFolderDialogContext::BrowseForExportTargetDirectory,
            AppMessage::ExportTargetDirectoryBrowseSubmitted,
            AppMessage::ExportTargetDirectoryBrowseError,
        );
    }

    if old.viewflags.sources_add_fs_fields_valid != new.viewflags.sources_add_fs_fields_valid {
        view.sources_add_fs_add_button
            .set_sensitive(new.viewflags.sources_add_fs_fields_valid);
    }

    if old.sources != new.sources {
        update_sources_list(model_ptr.clone(), new.clone(), view);
    }

    if old.viewvalues.sources_sample_count != new.viewvalues.sources_sample_count {
        for uuid in new.viewvalues.sources_sample_count.keys() {
            if let Some(count_label) = gtk_find_child_by_builder_id::<gtk::Label>(
                &view.sources_list.get(),
                &format!("{uuid}-count-label"),
            ) {
                count_label.set_text(&format!(
                    "({} samples)",
                    new.viewvalues.sources_sample_count.get(uuid).unwrap()
                ));
            }
        }
    }

    if old.samplelist_selected_sample != new.samplelist_selected_sample {
        update_samples_sidebar(model_ptr.clone(), new.clone(), view);
    }

    if old.viewflags.samples_sidebar_add_to_prev_enabled
        != new.viewflags.samples_sidebar_add_to_prev_enabled
    {
        view.samples_sidebar_add_to_prev_button
            .set_visible(new.viewflags.samples_sidebar_add_to_prev_enabled);
    }

    if old.sets_most_recently_used_uuid != new.sets_most_recently_used_uuid {
        if let Some(mru) = &new.sets_most_recently_used_uuid {
            if let Some((_, set)) = new.sets.iter().find(|(uuid, _set)| *uuid == mru) {
                view.samples_sidebar_add_to_prev_button
                    .set_label(&format!("Add to '{}'", set.name()));
            }
        }
    }

    if old.sets_selected_set != new.sets_selected_set {
        update_samplesets_detail(model_ptr.clone(), new.clone(), view);
    }

    if old.sets != new.sets {
        update_samplesets_list(model_ptr.clone(), new.clone(), view);
        update_samplesets_detail(model_ptr.clone(), new.clone(), view);

        if new.samplelist_selected_sample.is_some() {
            update_samples_sidebar(model_ptr.clone(), new.clone(), view);
        }
    }

    if old.viewflags.sets_export_enabled != new.viewflags.sets_export_enabled {
        view.sets_details_export_button
            .set_sensitive(new.viewflags.sets_export_enabled);
    }

    if old.sets_export_state != new.sets_export_state {
        match new.sets_export_state {
            Some(model::ExportState::Exporting) => {
                if let Some(dv) = &new.viewvalues.sets_export_dialog_view {
                    dv.window.close();
                    view.progress_popup.set_visible(true);
                }
            }

            Some(model::ExportState::Finished) => {
                view.progress_popup.set_visible(false);
            }

            None => (),
        }
    }

    if old.sets_export_progress != new.sets_export_progress {
        if let Some((n, m)) = &new.sets_export_progress {
            view.progress_popup_progress_bar
                .set_text(Some(format!("Exporting {n}/{m}").as_str()));

            view.progress_popup_progress_bar
                .set_fraction(*n as f64 / *m as f64);
        }
    }

    if old.drum_machine != new.drum_machine {
        update_drum_machine_view(new);
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
                    .with_spec(
                        AudioSpec::new(config.output_samplerate_hz, 2).unwrap_or_else(|_| {
                            log::log!(
                                log::Level::Error,
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
                                    log::Level::Error,
                                    "Invalid buffer size in config, using default"
                                );
                                NonZeroNumFrames::new(1024).unwrap()
                            }),
                    ),
            ),
        ));

        let view = AsampoView::new(app);

        let model = AppModel::new(
            config,
            None,
            Some(tx.clone()),
            Some(audiothread_handle.clone()),
        );
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

        view.present();

        // timer for AppMessage::TimerTick
        gtk::glib::timeout_add_seconds_local(
            1,
            clone!(@strong model_ptr, @strong view => move || {
                update(model_ptr.clone(), &view, AppMessage::TimerTick);
                gtk::glib::ControlFlow::Continue
            }),
        );

        // timer for async/thread messaging
        gtk::glib::timeout_add_local(
            std::time::Duration::from_millis(50),
            clone!(@strong model_ptr, @strong view => move || {
                let model = model_ptr.take().unwrap();
                let export_job_rx = model.export_job_rx.clone();
                let sources_loading = model.sources_loading.clone();
                model_ptr.set(Some(model));

                if let Some(rx) = export_job_rx {
                    loop {
                        match rx.try_recv() {
                            Ok(m) => update(
                                model_ptr.clone(),
                                &view,
                                AppMessage::ExportJobMessage(m)
                            ),

                            Err(e) => {
                                match e {
                                    mpsc::TryRecvError::Empty => (),
                                    mpsc::TryRecvError::Disconnected =>
                                        update(
                                            model_ptr.clone(),
                                            &view,
                                            AppMessage::ExportJobDisconnected
                                        ),
                                }

                                break
                            },
                        }
                    }
                }

                for uuid in sources_loading.keys() {
                    let recv = sources_loading.get(uuid).unwrap();

                    match recv.try_recv() {
                        Ok(message) => {
                            let mut messages = vec![message];
                            messages.extend(recv.try_iter());

                            update(
                                model_ptr.clone(),
                                &view,
                                AppMessage::SourceLoadingMessage(*uuid, messages)
                            );
                        }

                        Err(e) => {
                            match e {
                                mpsc::TryRecvError::Empty => (),
                                mpsc::TryRecvError::Disconnected => {
                                    update(
                                        model_ptr.clone(),
                                        &view,
                                        AppMessage::SourceLoadingDisconnected(*uuid)
                                    );
                                },
                            }
                        }
                    };
                }

                gtk::glib::ControlFlow::Continue
            }),
        );

        gtk::glib::timeout_add_local(
            std::time::Duration::from_millis(4),
            clone!(@strong model_ptr, @strong view => move || {
                let model = model_ptr.take().unwrap();

                let mut event: Option<DrumkitSequenceEvent> = None;

                if let Some(event_rx) = &model.drum_machine.event_rx {
                    match event_rx.borrow_mut().latest() {
                        Some(ev) if model.drum_machine.event_latest.is_none() ||
                            ev.step != model.drum_machine.event_latest.as_ref().unwrap().step =>
                        {
                            event = Some(ev.clone());
                        }
                        _ => (),
                    }
                }

                model_ptr.replace(Some(model));

                if let Some(ev) = event {
                    update(
                        model_ptr.clone(),
                        &view,
                        AppMessage::DrumMachinePlaybackEvent(ev.clone())
                    );
                }

                gtk::glib::ControlFlow::Continue
            }),
        );
    });

    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutils::savefile_for_test;

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
            &AppModel::new(AppConfig::default(), None, None, None)
                .add_source(src)
                .unwrap(),
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
