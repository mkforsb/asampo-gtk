// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{
    io::BufReader,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use audiothread::{SourceMatcher, SourceType};
use gtk::{gdk::ModifierType, DialogError};
use libasampo::{
    samples::{Sample, SampleOps},
    samplesets::{
        export::{Conversion, ExportJob, ExportJobMessage},
        BaseSampleSet, DrumkitLabel, SampleSet, SampleSetOps,
    },
    sequences::{drumkit_render_thread, DrumkitSequence, NoteLength, StepSequenceOps, TimeSpec},
    sources::SourceOps,
};

use crate::{
    appmessage::AppMessage,
    config::SamplePlaybackBehavior,
    configfile::ConfigFile,
    labels::DRUM_LABELS,
    model::{AppModel, DrumMachinePlaybackState, ExportKind, ExportState, Mirroring},
    savefile::Savefile,
    view::dialogs::{InputDialogContext, SelectFolderDialogContext},
    ErrorWithEffect,
};

fn play_sample(model: &AppModel, sample: &Sample) -> Result<(), anyhow::Error> {
    let stream = model
        .source(
            *sample
                .source_uuid()
                .ok_or(anyhow!("Sample missing source UUID"))?,
        )?
        .stream(sample)?;

    if model.config().sample_playback_behavior == SamplePlaybackBehavior::PlaySingleSample {
        model
            .audiothread_send(audiothread::Message::DropAllMatching(
                SourceMatcher::new().match_type(SourceType::SymphoniaSource),
            ))
            .map_err(|e| anyhow!("Send error on audiothread control channel: {e}"))?;
    }

    model
        .audiothread_send(audiothread::Message::PlaySymphoniaSource(
            audiothread::SymphoniaSource::from_buf_reader(BufReader::new(stream))?,
        ))
        .map_err(|e| anyhow!("Send error on audiothread control channel: {e}"))?;

    Ok(())
}

pub fn update_model(model: AppModel, message: AppMessage) -> Result<AppModel, anyhow::Error> {
    match message {
        AppMessage::TimerTick => {
            if model.has_sources_loading() {
                model.populate_samples_listmodel();
            }

            if model.reached_config_save_timeout() {
                let config = model.config().clone();

                log::log!(
                    log::Level::Info,
                    "Saving config to {:?}",
                    config.config_save_path
                );
                ConfigFile::save(&config, &config.config_save_path)?;

                log::log!(log::Level::Info, "Respawning audiothread with new config");
                Ok(model.reconfigure(config)?.clear_config_save_timeout())
            } else {
                Ok(model)
            }
        }

        AppMessage::SettingsOutputSampleRateChanged(choice) => {
            let new_config = model.config().clone().with_samplerate_choice(choice);

            Ok(model
                .set_latency_approx_label_by_config(&new_config)
                .set_config(new_config)
                .set_config_save_timeout(Instant::now() + Duration::from_secs(3)))
        }

        AppMessage::SettingsBufferSizeChanged(samples) => {
            let new_config = model.config().clone().with_buffer_size(samples);

            Ok(model
                .set_latency_approx_label_by_config(&new_config)
                .set_config(new_config)
                .set_config_save_timeout(Instant::now() + Duration::from_secs(3)))
        }

        AppMessage::SettingsSampleRateConversionQualityChanged(choice) => {
            let new_config = model
                .config()
                .clone()
                .with_conversion_quality_choice(choice);

            Ok(model
                .set_config(new_config)
                .set_config_save_timeout(Instant::now() + Duration::from_secs(3)))
        }

        AppMessage::SettingsSamplePlaybackBehaviorChanged(choice) => {
            let new_config = model
                .config()
                .clone()
                .with_sample_playback_behavior_choice(choice);

            Ok(model
                .set_config(new_config)
                .set_config_save_timeout(Instant::now() + Duration::from_secs(3)))
        }

        AppMessage::AddFilesystemSourceNameChanged(text) => Ok(model
            .set_add_fs_source_name(text)
            .validate_add_fs_source_fields()),

        AppMessage::AddFilesystemSourcePathChanged(text) => Ok(model
            .set_add_fs_source_path(text)
            .validate_add_fs_source_fields()),

        AppMessage::AddFilesystemSourcePathBrowseClicked => {
            Ok(model.signal_add_fs_source_begin_browse())
        }

        AppMessage::AddFilesystemSourcePathBrowseSubmitted(text) => {
            Ok(match Path::new(&text).file_name() {
                Some(filename) => model.set_add_fs_source_name_if_empty(
                    filename
                        .to_str()
                        .ok_or(anyhow!("Path contains invalid UTF-8"))?,
                ),
                None => model,
            }
            .set_add_fs_source_path(text)
            .validate_add_fs_source_fields())
        }

        AppMessage::AddFilesystemSourcePathBrowseError(error) => {
            log::log!(log::Level::Debug, "Error browsing for folder: {error:?}");

            Ok(model)
        }

        AppMessage::AddFilesystemSourceExtensionsChanged(text) => Ok(model
            .set_add_fs_source_extensions(text)
            .validate_add_fs_source_fields()),

        AppMessage::AddFilesystemSourceClicked => Ok(model
            .commit_file_system_source()?
            .tap(AppModel::populate_samples_listmodel)),

        AppMessage::SourceLoadingMessage(uuid, messages) => {
            model.handle_source_loader(uuid, messages)
        }

        AppMessage::SourceLoadingDisconnected(uuid) => {
            model.populate_samples_listmodel();
            model.remove_source_loader(uuid)
        }

        AppMessage::SampleListSampleSelected(index) => {
            let sample = model.get_listed_sample(index)?;
            play_sample(&model, &sample)?;

            Ok(model.set_selected_sample(Some(sample)))
        }

        AppMessage::SamplesFilterChanged(text) => Ok(model
            .set_samples_list_filter(text)
            .tap(AppModel::populate_samples_listmodel)),

        AppMessage::SampleSidebarAddToSetClicked => {
            Ok(model.signal_add_sample_to_set_show_dialog())
        }

        AppMessage::DeleteSampleFromSetClicked(sample, set_uuid) => {
            let model = model.remove_from_set(&sample, set_uuid)?;

            if model
                .drum_machine_loaded_sampleset()
                .is_some_and(|set| set.uuid() == set_uuid)
            {
                Ok(model.signal_sampleset_loaded_edit_show_dialog())
            } else {
                Ok(model)
            }
        }

        AppMessage::SampleSidebarAddToMostRecentlyUsedSetClicked => {
            let sample = model
                .selected_sample()
                .ok_or(anyhow!("No sample selected"))?
                .clone();

            let set_uuid = model
                .set_most_recently_added_to()
                .ok_or(anyhow!("No sample set recently added to"))?;

            let model = model.add_to_set(sample, set_uuid)?;

            if model
                .drum_machine_loaded_sampleset()
                .is_some_and(|set| set.uuid() == set_uuid)
            {
                Ok(model.signal_sampleset_loaded_edit_show_dialog())
            } else {
                Ok(model)
            }
        }

        AppMessage::SourceEnabled(uuid) => Ok(model
            .reset_source_sample_count(uuid)?
            .enable_source(uuid)?
            .tap(AppModel::populate_samples_listmodel)),

        AppMessage::SourceDisabled(uuid) => Ok(model
            .disable_source(uuid)?
            .tap(AppModel::populate_samples_listmodel)),

        AppMessage::SourceDeleteClicked(uuid) => Ok(model
            .remove_source(uuid)?
            .tap(AppModel::populate_samples_listmodel)),

        AppMessage::LoadFromSavefile(filename) => {
            log::log!(log::Level::Info, "Loading from {filename}");

            match Savefile::load(&filename) {
                Ok(loaded_savefile) => {
                    let mut result = model
                        .set_savefile_path(Some(filename))
                        .drum_machine_stop()?
                        .clear_sources()
                        .clear_sets()
                        .clear_sequences()
                        .load_sources(loaded_savefile.sources_domained()?)?
                        .load_sets(loaded_savefile.sets_domained()?)?
                        .load_sequences(loaded_savefile.sequences_domained()?)?
                        .set_selected_sequence(loaded_savefile.drum_machine_loaded_sequence())?
                        .set_selected_set_member(None);

                    if loaded_savefile.drum_machine_loaded_sequence().is_some() {
                        let sequence = result
                            .sequence(loaded_savefile.drum_machine_loaded_sequence().unwrap())?
                            .clone();

                        result = result.load_drum_machine_sequence(sequence)?;
                    } else {
                        result = result.clear_drum_machine_loaded_sequence();
                    }

                    result = result.set_drum_machine_sequence(
                        loaded_savefile.drum_machine_sequence_domained()?,
                        Mirroring::Mirror,
                    )?;

                    if loaded_savefile.drum_machine_loaded_sampleset().is_some() {
                        let set = result
                            .set(loaded_savefile.drum_machine_loaded_sampleset().unwrap())?
                            .clone();

                        result = result.load_drum_machine_sampleset(
                            set,
                            loaded_savefile.sources_domained()?,
                        )?;
                    } else {
                        result = result.clear_drum_machine_loaded_sampleset();
                    }

                    result = result.set_drum_machine_sampleset(
                        loaded_savefile.drum_machine_sampleset_domained()?,
                        loaded_savefile.sources_domained()?,
                        Mirroring::Mirror,
                    )?;

                    Ok(result)
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
                Ok(_) => Ok(model.set_savefile_path(Some(filename))),
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

        AppMessage::AddSampleSetClicked => Ok(model.signal_add_set_show_dialog()),

        AppMessage::InputDialogOpened(context) => match context {
            InputDialogContext::AddToSampleset => {
                Ok(model.clear_signal_add_sample_to_set_show_dialog())
            }

            InputDialogContext::CreateSampleSet => Ok(model.clear_signal_add_set_show_dialog()),

            InputDialogContext::CreateEmptySequence => {
                Ok(model.clear_signal_create_sequence_show_dialog())
            }

            InputDialogContext::SaveDrumMachineSequenceAs => {
                Ok(model.clear_signal_sequence_save_as_show_dialog())
            }

            InputDialogContext::SaveDrumMachineSampleSetAs => {
                Ok(model.clear_signal_sampleset_save_as_show_dialog())
            }
        },

        AppMessage::InputDialogCanceled(_context) => Ok(model),

        AppMessage::InputDialogSubmitted(context, text) => match context {
            InputDialogContext::AddToSampleset => {
                let (model, set_uuid) = AppModel::get_or_create_set(model, text)?;
                let sample = model
                    .selected_sample()
                    .ok_or(anyhow!("No sample selected"))?
                    .clone();

                let model = model
                    .add_to_set(sample, set_uuid)?
                    .set_add_to_prev_set_enabled(true);

                if model
                    .drum_machine_loaded_sampleset()
                    .is_some_and(|set| set.uuid() == set_uuid)
                {
                    Ok(model.signal_sampleset_loaded_edit_show_dialog())
                } else {
                    Ok(model)
                }
            }

            InputDialogContext::CreateSampleSet => {
                model.add_set(SampleSet::BaseSampleSet(BaseSampleSet::new(text)))
            }

            InputDialogContext::CreateEmptySequence => model.add_sequence(
                DrumkitSequence::new_named(text, TimeSpec::new(120, 4, 4)?, NoteLength::Sixteenth),
            ),

            InputDialogContext::SaveDrumMachineSequenceAs => {
                let mut sequence = DrumkitSequence::new_from(model.drum_machine_model().sequence());
                sequence.set_name(text.clone());

                model
                    .add_sequence(sequence.clone())?
                    .swap_drum_machine_sequence(sequence.clone())
                    .set_selected_sequence(Some(sequence.uuid()))
            }

            InputDialogContext::SaveDrumMachineSampleSetAs => {
                let mut set = SampleSet::BaseSampleSet(BaseSampleSet::new(text.clone()));

                for sample in model.drum_machine_sampleset().list() {
                    set.add(
                        model.source(
                            *sample
                                .source_uuid()
                                .ok_or(anyhow!("Sample missing source UUID"))?,
                        )?,
                        sample.clone(),
                    )?;
                    set.set_label::<DrumkitLabel, Option<DrumkitLabel>>(
                        sample,
                        model
                            .drum_machine_sampleset()
                            .get_label::<DrumkitLabel>(sample)?,
                    )?;
                }

                Ok(model
                    .add_set(set.clone())?
                    .swap_drum_machine_sampleset(set.clone()))
            }
        },

        // TODO: replace with function pointer, just like "ok" and "cancel" for input dialog?
        AppMessage::SelectFolderDialogOpened(context) => match context {
            SelectFolderDialogContext::BrowseForFilesystemSource => {
                Ok(model.clear_signal_add_fs_source_begin_browse())
            }

            SelectFolderDialogContext::BrowseForExportTargetDirectory => {
                Ok(model.clear_signal_export_begin_browse())
            }
        },

        AppMessage::SampleSetSelected(uuid) => {
            if Some(uuid) != model.selected_set() {
                let len = model.set(uuid)?.len();

                Ok(model
                    .set_set_export_enabled(len > 0)
                    .set_set_load_in_drum_machine_enabled(len > 0)
                    .set_selected_set(Some(uuid))?
                    .set_selected_set_member(None))
            } else {
                Ok(model)
            }
        }

        AppMessage::SampleSetSampleSelected(sample) => {
            play_sample(&model, &sample)?;
            Ok(model.set_selected_set_member(Some(sample)))
        }

        AppMessage::SampleSetSampleLabelChanged(sample, label) => {
            let set_uuid = model.selected_set().ok_or(anyhow!("No set selected"))?;
            let set = model.set(set_uuid)?;

            let model = if let Some(prev_assigned_label) = set
                .list()
                .iter()
                .find(|s| set.get_label::<DrumkitLabel>(s).is_ok_and(|sl| sl == label))
            {
                let prev_sample = (*prev_assigned_label).clone();

                model
                    .set_sample_label(set_uuid, prev_sample, None)?
                    .set_sample_label(set_uuid, sample, label)
            } else {
                model.set_sample_label(set_uuid, sample, label)
            }?;

            if model
                .drum_machine_loaded_sampleset()
                .is_some_and(|set| set.uuid() == set_uuid)
            {
                Ok(model.signal_sampleset_loaded_edit_show_dialog())
            } else {
                Ok(model)
            }
        }

        AppMessage::SampleSetDetailsLoadInDrumMachineClicked => {
            if model.drum_machine_model().is_sampleset_modified() {
                if model.drum_machine_loaded_sampleset().is_some() {
                    Ok(model.signal_sampleset_load_show_confirm_save_dialog())
                } else {
                    Ok(model.signal_sampleset_load_show_confirm_abandon_dialog())
                }
            } else {
                let set = model
                    .set(model.selected_set().ok_or(anyhow!("No set selected"))?)?
                    .clone();
                let sources = model.sources_list().iter().cloned().cloned().collect();

                Ok(model.load_drum_machine_sampleset(set, sources)?)
            }
        }

        AppMessage::SampleSetDetailsExportClicked => Ok(model.signal_export_show_dialog()),

        AppMessage::ExportDialogOpened(dialogview) => Ok(model
            .clear_signal_export_show_dialog()
            .set_main_view_sensitive(false)
            .set_export_dialog_view(Some(dialogview))),

        AppMessage::ExportDialogClosed => Ok(model
            .set_export_dialog_view(None)
            .set_main_view_sensitive(true)),

        AppMessage::ExportTargetDirectoryChanged(text) => Ok(model
            .set_export_fields_valid(!text.is_empty())
            .set_export_target_dir(text)),

        AppMessage::ExportTargetDirectoryBrowseClicked => Ok(model.signal_export_begin_browse()),

        AppMessage::ExportTargetDirectoryBrowseSubmitted(text) => {
            Ok(model.set_export_target_dir(text))
        }

        AppMessage::ExportTargetDirectoryBrowseError(_e) => Ok(model),

        AppMessage::PerformExportClicked => {
            use libasampo::samplesets::export::{RateConversionQuality, WavSampleFormat, WavSpec};

            let set = model
                .set(
                    model
                        .selected_set()
                        .ok_or(anyhow!("No sample set selected"))?,
                )?
                .clone();

            let num_samples = set.len();
            let (export_tx, export_rx) = std::sync::mpsc::channel::<ExportJobMessage>();
            let sources = model.sources_map().clone();
            let target_dir = model.export_target_dir().clone();
            let export_kind = model.export_kind().clone();

            std::thread::spawn(move || {
                let job = ExportJob::new(
                    target_dir,
                    match export_kind {
                        ExportKind::PlainCopy => None,
                        ExportKind::Conversion => Some(Conversion::Wav(
                            WavSpec {
                                channels: 2,
                                sample_rate: 44100,
                                bits_per_sample: 16,
                                sample_format: WavSampleFormat::Int,
                            },
                            Some(RateConversionQuality::High),
                        )),
                    },
                );

                job.perform(&set, &sources, Some(export_tx));
            });

            Ok(model
                .set_export_state(Some(ExportState::Exporting))
                .set_export_job_rx(Some(export_rx))
                .init_export_progress(num_samples))
        }

        AppMessage::PlainCopyExportSelected => Ok(model.set_export_kind(ExportKind::PlainCopy)),
        AppMessage::ConversionExportSelected => Ok(model.set_export_kind(ExportKind::Conversion)),

        AppMessage::ExportJobMessage(message) => match message {
            ExportJobMessage::ItemsCompleted(n) => model.set_export_items_completed(n),
            ExportJobMessage::Error(e) => Err(e.into()),
            ExportJobMessage::Finished => Ok(model
                .set_export_state(Some(ExportState::Finished))
                .set_export_job_rx(None)
                .reset_export_progress()),
        },

        AppMessage::ExportJobDisconnected => Ok(model.set_export_job_rx(None)),

        AppMessage::StopAllSoundButtonClicked => {
            match model.audiothread_send(audiothread::Message::DropAllMatching(
                SourceMatcher::default().match_type(SourceType::SymphoniaSource),
            )) {
                Ok(_) => (),
                Err(e) => log::log!(log::Level::Error, "Stop all sounds error: {e}"),
            }

            model.drum_machine_stop()
        }

        AppMessage::DrumMachineTempoChanged(tempo) => {
            model.set_drum_machine_tempo(tempo, Mirroring::Mirror)
        }

        AppMessage::DrumMachineSwingChanged(swing) => {
            model.set_drum_machine_swing(swing as f64 / 100.0, Mirroring::Mirror)
        }

        AppMessage::DrumMachinePlayClicked => match model.drum_machine_playback_state() {
            DrumMachinePlaybackState::Paused | DrumMachinePlaybackState::Stopped => {
                model.drum_machine_play()
            }
            DrumMachinePlaybackState::Playing => model.drum_machine_pause(),
        },

        AppMessage::DrumMachineStopClicked => model.drum_machine_stop(),

        AppMessage::DrumMachineBackClicked => {
            model.drum_machine_rewind()?;
            Ok(model)
        }

        AppMessage::DrumMachineSaveSequenceClicked => {
            let sequence = model.drum_machine_model().sequence().clone();

            let position = model
                .sequences_list()
                .iter()
                .position(|seq| seq.uuid() == sequence.uuid())
                .ok_or(anyhow!("Sequence not found: UUID not present"))?;

            model
                .remove_sequence(sequence.uuid())?
                .insert_sequence(sequence, position)?
                .commit_drum_machine_sequence()
        }

        AppMessage::DrumMachineSaveSequenceAsClicked => {
            Ok(model.signal_sequence_save_as_show_dialog())
        }

        AppMessage::DrumMachineClearSequenceClicked => {
            Ok(model.signal_sequence_clear_show_confirm_dialog())
        }

        AppMessage::DrumMachineSaveSampleSetClicked => {
            let set = model.drum_machine_model().sampleset().clone();

            let position = model
                .sets_list()
                .iter()
                .position(|x| x.uuid() == set.uuid())
                .ok_or(anyhow!("Sample set not found: UUID not present"))?;

            model
                .remove_set(set.uuid())?
                .insert_set(set, position)?
                .commit_drum_machine_sampleset()
        }
        AppMessage::DrumMachineSaveSampleSetAsClicked => {
            Ok(model.signal_sampleset_save_as_show_dialog())
        }

        AppMessage::DrumMachineClearSampleSetClicked => {
            Ok(model.signal_sampleset_clear_show_confirm_dialog())
        }

        AppMessage::DrumMachinePadClicked(n) => {
            let label = DRUM_LABELS[n].1;
            let samples = model.drum_machine_sampleset().list();

            let sample = samples
                .iter()
                .cloned()
                .find(|&sample| {
                    model
                        .drum_machine_sampleset()
                        .get_label::<DrumkitLabel>(sample)
                        .is_ok_and(|val| val == Some(label))
                })
                .cloned();

            if let Some(sample) = sample {
                play_sample(&model, &sample)?;
            }

            model.set_activated_drum_machine_pad(n)
        }

        AppMessage::DrumMachinePartClicked(n, mods) => {
            if mods.contains(ModifierType::SHIFT_MASK) {
                model
                    .truncate_drum_machine_parts_to(n)?
                    .set_activated_drum_machine_part(n)
            } else {
                model.set_activated_drum_machine_part(n)
            }
        }
        AppMessage::DrumMachineStepClicked(n) => {
            let amp = 0.5f32;
            let mut new_sequence = model.drum_machine_sequence().clone();
            let label = DRUM_LABELS[model.activated_drum_machine_pad()].1;
            let offset = model.activated_drum_machine_part() * 16;
            let target_step = n + offset;

            if new_sequence
                .labels_at_step(target_step)
                .ok_or(anyhow!("Drum machine sequence has no step {target_step}"))?
                .contains(&label)
            {
                new_sequence.unset_step_trigger(
                    target_step,
                    DRUM_LABELS[model.activated_drum_machine_pad()].1,
                );

                if model.is_drum_machine_render_thread_active() {
                    model
                        .drum_machine_send(
                            drumkit_render_thread::Message::EditSequenceUnsetStepTrigger {
                                step: target_step,
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
                let set = model.drum_machine_sampleset();
                if let Some(sample) = set.list().iter().find(|s| {
                    set.get_label::<DrumkitLabel>(s)
                        .is_ok_and(|lb| lb == Some(label))
                }) {
                    play_sample(&model, sample)?;
                }

                new_sequence.set_step_trigger(target_step, label, amp);

                if model.is_drum_machine_render_thread_active() {
                    model
                        .drum_machine_send(
                            drumkit_render_thread::Message::EditSequenceSetStepTrigger {
                                step: target_step,
                                label,
                                amp,
                            },
                        )
                        .map_err(|e| {
                            anyhow!(
                                "Failed sending update event to drum sequence render thread: {e}"
                            )
                        })?;
                }
            }

            model.set_drum_machine_sequence(new_sequence, Mirroring::Off)
        }

        AppMessage::DrumMachinePlaybackEvent(event) => {
            Ok(model.set_latest_drum_machine_event(Some(event)))
        }

        AppMessage::AssignSampleToPadClicked(n) => {
            let sample = model
                .selected_sample()
                .ok_or(anyhow!("No sample selected"))?
                .clone();

            let source = model
                .source(
                    *sample
                        .source_uuid()
                        .ok_or(anyhow!("Sample missing source UUID"))?,
                )?
                .clone();

            let label = DRUM_LABELS.get(n).ok_or(anyhow!("No such label"))?.1;

            model.assign_drum_pad(&source, sample, label)
        }

        AppMessage::SequenceSelected(uuid) => {
            let model = model.set_selected_sequence(Some(uuid))?;

            if model.drum_machine_model().is_sequence_modified() {
                if model.drum_machine_loaded_sequence().is_some() {
                    Ok(model.signal_sequence_load_show_confirm_save_dialog())
                } else {
                    Ok(model.signal_sequence_load_show_confirm_abandon_dialog())
                }
            } else {
                let sequence = model.sequence(uuid)?.clone();
                Ok(model.load_drum_machine_sequence(sequence)?)
            }
        }

        AppMessage::AddSequenceClicked => Ok(model.signal_create_sequence_show_dialog()),

        AppMessage::LoadSequenceConfirmSaveDialogOpened => Ok(model
            .set_main_view_sensitive(false)
            .clear_signal_sequence_load_show_confirm_save_dialog()),

        AppMessage::LoadSequenceConfirmSaveChanges => {
            let sequence_to_save = model.drum_machine_model().sequence().clone();

            let position = model
                .sequences_list()
                .iter()
                .position(|seq| seq.uuid() == sequence_to_save.uuid())
                .ok_or(anyhow!("Sequence not found: UUID not present"))?;

            let model = model
                .remove_sequence(sequence_to_save.uuid())?
                .insert_sequence(sequence_to_save, position)?;

            let sequence_uuid_to_load = model
                .selected_sequence()
                .ok_or(anyhow!("Cannot finish loading, no sequence selected"))?;

            let sequence_to_load = model.sequence(sequence_uuid_to_load)?.clone();

            Ok(model
                .load_drum_machine_sequence(sequence_to_load)?
                .set_main_view_sensitive(true))
        }

        AppMessage::LoadSequenceConfirmDiscardChanges => {
            let sequence_to_load = model
                .sequence(
                    model
                        .selected_sequence()
                        .ok_or(anyhow!("Cannot finish loading, no sequence selected"))?,
                )?
                .clone();

            Ok(model
                .load_drum_machine_sequence(sequence_to_load)?
                .set_main_view_sensitive(true))
        }

        AppMessage::LoadSequenceCancelSave => {
            let loaded_uuid = model
                .drum_machine_model()
                .loaded_sequence()
                .ok_or(anyhow!("No sequence loaded"))?
                .uuid();

            Ok(model
                .set_selected_sequence(Some(loaded_uuid))?
                .set_main_view_sensitive(true))
        }

        AppMessage::LoadSequenceConfirmAbandonDialogOpened => Ok(model
            .set_main_view_sensitive(false)
            .clear_signal_sequence_load_show_confirm_abandon_dialog()),

        AppMessage::LoadSequenceConfirmAbandon => {
            let sequence_to_load = model
                .sequence(
                    model
                        .selected_sequence()
                        .ok_or(anyhow!("Cannot finish loading, no sequence selected"))?,
                )?
                .clone();

            Ok(model
                .load_drum_machine_sequence(sequence_to_load)?
                .set_main_view_sensitive(true))
        }

        AppMessage::LoadSequenceCancelAbandon => Ok(model
            .set_selected_sequence(None)?
            .set_main_view_sensitive(true)),

        AppMessage::LoadSequenceConfirmDialogError(e) => {
            log::log!(log::Level::Error, "{e}");
            Ok(model.set_main_view_sensitive(true))
        }

        AppMessage::ClearSequenceConfirmDialogOpened => Ok(model
            .set_main_view_sensitive(false)
            .clear_signal_sequence_clear_show_confirm_dialog()),

        AppMessage::ClearSequenceConfirmDialogError(e) => {
            log::log!(log::Level::Error, "{e}");
            Ok(model.set_main_view_sensitive(true))
        }

        AppMessage::ClearSequenceConfirm => Ok(model
            .clear_drum_machine_sequence()?
            .set_selected_sequence(None)?
            .set_main_view_sensitive(true)),

        AppMessage::ClearSequenceCancel => Ok(model.set_main_view_sensitive(true)),

        AppMessage::ClearSampleSetConfirmDialogOpened => Ok(model
            .set_main_view_sensitive(false)
            .clear_signal_sampleset_clear_show_confirm_dialog()),

        AppMessage::ClearSampleSetConfirmDialogError(e) => {
            log::log!(log::Level::Error, "{e}");
            Ok(model.set_main_view_sensitive(true))
        }

        AppMessage::ClearSampleSetConfirm => Ok(model
            .clear_drum_machine_sampleset()?
            .set_selected_sequence(None)?
            .set_main_view_sensitive(true)),

        AppMessage::ClearSampleSetCancel => Ok(model.set_main_view_sensitive(true)),

        AppMessage::LoadSampleSetConfirmSaveDialogOpened => {
            Ok(model.clear_signal_sampleset_load_show_confirm_save_dialog())
        }

        AppMessage::LoadSampleSetConfirmAbandonDialogOpened => {
            Ok(model.clear_signal_sampleset_load_show_confirm_abandon_dialog())
        }

        AppMessage::LoadSampleSetCancelSave => Ok(model),

        AppMessage::LoadSampleSetCancelAbandon => Ok(model),

        AppMessage::LoadSampleSetConfirmAbandon => {
            let set = model
                .set(model.selected_set().ok_or(anyhow!("No set selected"))?)?
                .clone();
            let sources = model.sources_list().iter().cloned().cloned().collect();

            Ok(model.load_drum_machine_sampleset(set, sources)?)
        }

        AppMessage::LoadSampleSetConfirmDiscardChanges => {
            let set = model
                .set(model.selected_set().ok_or(anyhow!("No set selected"))?)?
                .clone();
            let sources = model.sources_list().iter().cloned().cloned().collect();

            Ok(model.load_drum_machine_sampleset(set, sources)?)
        }

        AppMessage::LoadSampleSetConfirmSaveChanges => {
            let set_to_save = model.drum_machine_model().sampleset().clone();

            let position = model
                .sets_list()
                .iter()
                .position(|set| set.uuid() == set_to_save.uuid())
                .ok_or(anyhow!("Set not found: UUID not present"))?;

            let model = model
                .remove_set(set_to_save.uuid())?
                .insert_set(set_to_save, position)?;

            let set_uuid_to_load = model
                .selected_set()
                .ok_or(anyhow!("Cannot finish loading, no sequence selected"))?;

            let set_to_load = model.set(set_uuid_to_load)?.clone();
            let sources = model.sources_list().iter().cloned().cloned().collect();

            Ok(model
                .load_drum_machine_sampleset(set_to_load, sources)?
                .set_main_view_sensitive(true))
        }

        AppMessage::LoadSampleSetConfirmDialogError(e) => {
            log::log!(log::Level::Error, "Confirm dialog error: {e}");
            Ok(model)
        }

        AppMessage::SynchronizeSampleSetDialogOpened => {
            Ok(model.clear_signal_sampleset_loaded_edit_show_dialog())
        }

        AppMessage::SynchronizeSampleSetDialogError(e) => {
            log::log!(log::Level::Error, "Dialog error: {e}");
            Ok(model)
        }

        AppMessage::SynchronizeSampleSetConfirm => {
            let set = model
                .set(model.selected_set().ok_or(anyhow!("No set selected"))?)?
                .clone();
            let sources = model.sources_list().iter().cloned().cloned().collect();

            Ok(model.load_drum_machine_sampleset(set, sources)?)
        }

        AppMessage::SynchronizeSampleSetUnlink => Ok(model.clear_drum_machine_loaded_sampleset()),

        AppMessage::SynchronizeSampleSetCancel => {
            let set = model.drum_machine_sampleset().clone();
            let uuid = set.uuid();

            let position = model
                .sets_list()
                .iter()
                .position(|set| set.uuid() == uuid)
                .ok_or(anyhow!("Set not found: UUID not present"))?;

            Ok(model.remove_set(uuid)?.insert_set(set, position)?)
        }
    }
}
