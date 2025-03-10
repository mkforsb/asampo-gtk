// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use anyhow::anyhow;
use gtk::prelude::*;
use libasampo::{samplesets::SampleSetOps, sequences::StepSequenceOps};

use crate::{
    appmessage::AppMessage,
    model::{AppModel, AppModelPtr, ExportState, Signal},
    util::gtk_find_child_by_builder_id,
    view::{
        dialogs::{self, ButtonSpec, InputDialogContext, SelectFolderDialogContext},
        samples::update_samples_sidebar,
        sequences::{update_drum_machine_view, update_sequences_list},
        sets::{update_samplesets_detail, update_samplesets_list},
        sources::update_sources_list,
        AsampoView,
    },
};

pub fn update_view(model_ptr: AppModelPtr, old: AppModel, new: AppModel, view: &AsampoView) {
    macro_rules! maybe_update_text {
        ($viewexpr:expr, $fname:ident) => {
            if old.$fname() != new.$fname() && ($viewexpr).text() != *new.$fname() {
                ($viewexpr).set_text(&new.$fname());
            }
        };
    }

    macro_rules! is {
        ($model:ident, $signal:ident) => {
            $model.is_signalling(Signal::$signal)
        };
    }

    macro_rules! closer {
        ($message:expr) => {
            AppMessage::Sequence(vec![AppMessage::DialogClosed, $message])
        };
    }

    if is!(old, MainViewSensitive) != is!(new, MainViewSensitive) {
        view.set_sensitive(is!(new, MainViewSensitive));
    }

    maybe_update_text!(view.settings_latency_approx_label, latency_approx_label);
    maybe_update_text!(view.sources_add_fs_name_entry, add_fs_source_name);
    maybe_update_text!(view.sources_add_fs_path_entry, add_fs_source_path);
    maybe_update_text!(
        view.sources_add_fs_extensions_entry,
        add_fs_source_extensions
    );

    if let Some(dialogview) = new.export_dialog_view() {
        maybe_update_text!(dialogview.target_dir_entry, export_target_dir);

        if is!(old, SampleSetExportFieldsValid) != is!(new, SampleSetExportFieldsValid) {
            dialogview
                .export_button
                .set_sensitive(is!(new, SampleSetExportFieldsValid));
        }
    }

    if is!(new, ShowAddFilesystemSourceBrowseDialog) {
        dialogs::choose_folder(
            model_ptr.clone(),
            view,
            SelectFolderDialogContext::BrowseForFilesystemSource,
            |s| closer!(AppMessage::AddFilesystemSourcePathBrowseSubmitted(s)),
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Error browsing for folder: {e}").into()
                ))
            },
        );
    }

    if is!(new, ShowAddSampleToSetDialog) {
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

    if is!(new, ShowSampleSetCreateDialog) {
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

    if is!(new, ShowSampleSetDeleteDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            format!(
                "Really delete sample set '{}'?",
                new.set(
                    new.set_pending_deletion()
                        .expect("A set should be pending deletion")
                )
                .expect("The set should exist")
                .name()
            )
            .as_str(),
            "This action cannot be undone",
            vec![
                ButtonSpec::new("Ok", || closer!(AppMessage::SampleSetDeleteConfirmed)),
                ButtonSpec::new("Cancel", || closer!(AppMessage::SampleSetDeleteCanceled))
                    .set_as_cancel(),
            ],
            AppMessage::SampleSetDeleteDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        )
    }

    if is!(new, ShowSampleSetSynchronizationDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            "Synchronize changes with drum machine?",
            "A change was made to the sample set loaded in the drum machine. Unlink \
                if you want to treat these as two different sets. Cancel to roll back \
                the change.",
            vec![
                ButtonSpec::new("Synchronize", || {
                    closer!(AppMessage::SynchronizeSampleSetConfirm)
                })
                .set_as_default(),
                ButtonSpec::new("Unlink", || closer!(AppMessage::SynchronizeSampleSetUnlink)),
                ButtonSpec::new("Cancel", || closer!(AppMessage::SynchronizeSampleSetCancel))
                    .set_as_cancel(),
            ],
            AppMessage::SynchronizeSampleSetDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        );
    }

    if is!(new, ShowSequenceCreateDialog) {
        dialogs::input(
            model_ptr.clone(),
            view,
            InputDialogContext::CreateEmptySequence,
            "Add sequence",
            "Name of sequence:",
            "Name",
            "Add",
        );
    }

    if is!(new, ShowSequenceDeleteDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            format!(
                "Really delete sequence '{}'?",
                new.sequence(
                    new.sequence_pending_deletion()
                        .expect("A sequence should be pending deletion")
                )
                .expect("The sequence should exist")
                .name()
            )
            .as_str(),
            "This action cannot be undone",
            vec![
                ButtonSpec::new("Ok", || closer!(AppMessage::SequenceDeleteConfirmed)),
                ButtonSpec::new("Cancel", || closer!(AppMessage::SequenceDeleteCanceled))
                    .set_as_cancel(),
            ],
            AppMessage::SequenceDeleteDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        )
    }

    if is!(new, ShowSequenceSaveAsDialog) {
        dialogs::input(
            model_ptr.clone(),
            view,
            InputDialogContext::SaveDrumMachineSequenceAs,
            "Save sequence as",
            "Name of sequence:",
            "Name",
            "Save",
        );
    }

    if is!(new, ShowSampleSetSaveAsDialog) {
        dialogs::input(
            model_ptr.clone(),
            view,
            InputDialogContext::SaveDrumMachineSampleSetAs,
            "Save sample set as",
            "Name of set:",
            "Name",
            "Save",
        );
    }

    if is!(new, ShowSequenceSaveBeforeLoadDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            format!(
                "Save changes to sequence {}?",
                new.drum_machine_model()
                    .loaded_sequence()
                    .expect("There should be a loaded sequence")
                    .name()
            )
            .as_str(),
            "",
            vec![
                ButtonSpec::new("Save changes", || {
                    closer!(AppMessage::LoadSequenceConfirmSaveChanges)
                })
                .set_as_default(),
                ButtonSpec::new("Discard changes", || {
                    closer!(AppMessage::LoadSequenceConfirmDiscardChanges)
                }),
                ButtonSpec::new("Cancel", || closer!(AppMessage::LoadSequenceCancelSave))
                    .set_as_cancel(),
            ],
            AppMessage::LoadSequenceConfirmSaveDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        );
    }

    if is!(new, ShowSequenceConfirmAbandonDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            "Abandon unnamed sequence?",
            "This action cannot be undone",
            vec![
                ButtonSpec::new("Ok", || closer!(AppMessage::LoadSequenceConfirmAbandon)),
                ButtonSpec::new("Cancel", || closer!(AppMessage::LoadSequenceCancelAbandon))
                    .set_as_cancel(),
            ],
            AppMessage::LoadSequenceConfirmAbandonDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        )
    }

    if is!(new, ShowSampleSetSaveBeforeLoadDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            format!(
                "Save changes to sample set {}?",
                new.drum_machine_model()
                    .loaded_sampleset()
                    .expect("There should be a loaded sampleset")
                    .name()
            )
            .as_str(),
            "The sample set was loaded into the drum machine, and has been modified there",
            vec![
                ButtonSpec::new("Save changes", || {
                    closer!(AppMessage::LoadSampleSetConfirmSaveChanges)
                })
                .set_as_default(),
                ButtonSpec::new("Discard changes", || {
                    closer!(AppMessage::LoadSampleSetConfirmDiscardChanges)
                }),
                ButtonSpec::new("Cancel", || AppMessage::DialogClosed).set_as_cancel(),
            ],
            AppMessage::LoadSampleSetConfirmSaveDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        );
    }

    if is!(new, ShowSampleSetConfirmAbandonDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            "Abandon unnamed sample set?",
            "The drum machine contains an unnamed and unsaved sample set. \
                Abandoning this set cannot be undone.",
            vec![
                ButtonSpec::new("Ok", || closer!(AppMessage::LoadSampleSetConfirmAbandon)),
                ButtonSpec::new("Cancel", || AppMessage::DialogClosed).set_as_cancel(),
            ],
            AppMessage::LoadSampleSetConfirmAbandonDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        )
    }

    if is!(new, ShowSequenceConfirmClearDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            "Clear sequence?",
            "This action cannot be undone",
            vec![
                ButtonSpec::new("Ok", || closer!(AppMessage::ClearSequenceConfirm)),
                ButtonSpec::new("Cancel", || AppMessage::DialogClosed).set_as_cancel(),
            ],
            AppMessage::ClearSequenceConfirmDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        )
    }

    if is!(new, ShowSampleSetConfirmClearDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            "Clear sample set?",
            "This action cannot be undone",
            vec![
                ButtonSpec::new("Ok", || closer!(AppMessage::ClearSampleSetConfirm)),
                ButtonSpec::new("Cancel", || AppMessage::DialogClosed).set_as_cancel(),
            ],
            AppMessage::ClearSampleSetConfirmDialogOpened,
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Confirm dialog error: {e}").into()
                ))
            },
        )
    }

    if is!(new, ShowExportDialog) {
        dialogs::sampleset_export(model_ptr.clone(), view, new.clone());
    }

    if is!(new, ShowExportBrowseDialog) {
        dialogs::choose_folder(
            model_ptr.clone(),
            view,
            SelectFolderDialogContext::BrowseForExportTargetDirectory,
            AppMessage::ExportTargetDirectoryBrowseSubmitted,
            |e| AppMessage::LogError(anyhow!("Export browse dialog error: {e}").into()),
        );
    }

    if is!(new, ShowSaveBeforeQuitConfirmDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            "Save workspace before quitting?",
            "",
            vec![
                ButtonSpec::new("Save", || closer!(AppMessage::SaveAndQuitBegin)).set_as_default(),
                ButtonSpec::new("Do not save", || closer!(AppMessage::Quit)),
                ButtonSpec::new("Cancel", || AppMessage::DialogClosed).set_as_cancel(),
            ],
            AppMessage::SaveBeforeQuitConfirmDialogOpened,
            |e| closer!(AppMessage::LogError(anyhow!("Dialog error: {e:?}").into())),
        );
    }

    if is!(new, ShowSaveBeforeQuitSaveDialog) {
        dialogs::save(
            model_ptr.clone(),
            view,
            AppMessage::SaveBeforeQuitSaveDialogOpened,
            |s| closer!(AppMessage::SaveAndQuitFinish(s)),
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Save dialog error: {e}").into()
                ))
            },
        )
    }

    if is!(new, ShowSaveBeforeLoadConfirmDialog) {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            "Save workspace before loading another?",
            "",
            vec![
                ButtonSpec::new("Save", || closer!(AppMessage::SaveBeforeLoadPerformSave))
                    .set_as_default(),
                ButtonSpec::new("Do not save", || {
                    closer!(AppMessage::SaveBeforeLoadPerformLoad)
                }),
                ButtonSpec::new("Cancel", || AppMessage::DialogClosed).set_as_cancel(),
            ],
            AppMessage::SaveBeforeLoadConfirmDialogOpened,
            |e| closer!(AppMessage::LogError(anyhow!("Dialog error: {e:?}").into())),
        );
    }

    if is!(new, ShowSaveBeforeLoadSaveDialog) {
        dialogs::save(
            model_ptr.clone(),
            view,
            AppMessage::SaveBeforeLoadSaveDialogOpened,
            |s| {
                closer!(AppMessage::Sequence(vec![
                    AppMessage::SaveToSavefile(s),
                    AppMessage::SaveBeforeLoadPerformLoad
                ]))
            },
            |e| {
                closer!(AppMessage::LogError(
                    anyhow!("Save dialog error: {e}").into()
                ))
            },
        )
    }

    if is!(old, AddFilesystemSourceFieldsValid) != is!(new, AddFilesystemSourceFieldsValid) {
        view.sources_add_fs_add_button
            .set_sensitive(is!(new, AddFilesystemSourceFieldsValid));
    }

    if old.sources_map() != new.sources_map() {
        update_sources_list(model_ptr.clone(), new.clone(), view);
    }

    if old.sources_sample_count() != new.sources_sample_count() {
        for uuid in new.sources_sample_count().keys() {
            if let Some(count_label) = gtk_find_child_by_builder_id::<gtk::Label>(
                &view.sources_list.get(),
                &format!("{uuid}-count-label"),
            ) {
                count_label.set_text(&format!(
                    "({} samples)",
                    new.sources_sample_count().get(uuid).unwrap()
                ));
            }
        }
    }

    if old.selected_sample() != new.selected_sample() {
        update_samples_sidebar(model_ptr.clone(), new.clone(), view);
    }

    if is!(old, AddToPreviousSetEnabled) != is!(new, AddToPreviousSetEnabled) {
        view.samples_sidebar_add_to_prev_button
            .set_visible(is!(new, AddToPreviousSetEnabled));
    }

    if old.set_most_recently_added_to() != new.set_most_recently_added_to() {
        if let Some(mru) = &new.set_most_recently_added_to() {
            if let Ok(set) = new.set(*mru) {
                view.samples_sidebar_add_to_prev_button
                    .set_label(&format!("Add to '{}'", set.name()));
            }
        }
    }

    if old.selected_set() != new.selected_set() || old.sets_map() != new.sets_map() {
        update_samplesets_detail(model_ptr.clone(), &new, view);
    }

    if old.sets_map() != new.sets_map() {
        update_samplesets_list(model_ptr.clone(), new.clone(), view);

        if new.selected_sample().is_some() {
            update_samples_sidebar(model_ptr.clone(), new.clone(), view);
        }
    }

    if is!(old, LoadSetInDrumMachineEnabled) != is!(new, LoadSetInDrumMachineEnabled) {
        view.sets_details_load_drum_machine_button
            .set_sensitive(is!(new, LoadSetInDrumMachineEnabled));
    }

    if is!(old, SampleSetExportEnabled) != is!(new, SampleSetExportEnabled) {
        view.sets_details_export_button
            .set_sensitive(is!(new, SampleSetExportEnabled));
    }

    if old.export_state() != new.export_state() {
        match new.export_state() {
            Some(ExportState::Exporting) => {
                view.progress_popup.set_visible(true);
            }

            Some(ExportState::Finished) => {
                view.progress_popup.set_visible(false);
            }

            None => (),
        }
    }

    if old.export_progress() != new.export_progress() {
        if let Some((n, m)) = &new.export_progress() {
            view.progress_popup_progress_bar
                .set_text(Some(format!("Exporting {n}/{m}").as_str()));

            view.progress_popup_progress_bar
                .set_fraction(*n as f64 / *m as f64);
        }
    }

    if old.sequences_map() != new.sequences_map()
        || old.selected_sequence() != new.selected_sequence()
    {
        update_sequences_list(model_ptr.clone(), &new, view);
    }

    if new
        .drum_machine_model()
        .is_visibly_modified_vs(old.drum_machine_model())
    {
        update_drum_machine_view(&new);
    }

    if is!(new, QuitConfirmed) {
        view.destroy()
    }
}
