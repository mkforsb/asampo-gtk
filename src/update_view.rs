// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use anyhow::anyhow;
use gtk::prelude::*;
use libasampo::{samplesets::SampleSetOps, sequences::StepSequenceOps};

use crate::{
    appmessage::AppMessage,
    model::{AppModel, AppModelPtr, ExportState},
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

    if old.is_main_view_sensitive() != new.is_main_view_sensitive() {
        view.set_sensitive(new.is_main_view_sensitive());
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

        if old.are_export_fields_valid() != new.are_export_fields_valid() {
            dialogview
                .export_button
                .set_sensitive(new.are_export_fields_valid());
        }
    }

    if new.is_signalling_add_fs_source_begin_browse() {
        dialogs::choose_folder(
            model_ptr.clone(),
            view,
            SelectFolderDialogContext::BrowseForFilesystemSource,
            AppMessage::AddFilesystemSourcePathBrowseSubmitted,
            AppMessage::AddFilesystemSourcePathBrowseError,
        );
    }

    if new.is_signalling_add_sample_to_set_show_dialog() {
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

    if new.is_signalling_add_set_show_dialog() {
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

    if new.is_signalling_create_sequence_show_dialog() {
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

    if new.is_signalling_sequence_save_as_show_dialog() {
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

    if new.is_signalling_sequence_load_show_confirm_dialog() {
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
                    AppMessage::LoadSequenceConfirmSaveChanges
                })
                .set_as_default(),
                ButtonSpec::new("Discard changes", || {
                    AppMessage::LoadSequenceConfirmDiscardChanges
                }),
                ButtonSpec::new("Cancel", || AppMessage::LoadSequenceCancel).set_as_cancel(),
            ],
            AppMessage::LoadSequenceConfirmDialogOpened,
            |e| AppMessage::LoadSequenceConfirmDialogError(anyhow!("Confirm dialog error: {e:?}")),
        );
    }

    if new.is_signalling_sequence_clear_show_confirm_dialog() {
        dialogs::confirm(
            model_ptr.clone(),
            view,
            "Clear sequence?",
            "This action cannot be undone",
            vec![
                ButtonSpec::new("Ok", || AppMessage::ClearSequenceConfirm),
                ButtonSpec::new("Cancel", || AppMessage::ClearSequenceCancel).set_as_cancel(),
            ],
            AppMessage::ClearSequenceConfirmDialogOpened,
            |e| AppMessage::ClearSequenceConfirmDialogError(anyhow!("Confirm dialog error: {e:?}")),
        )
    }

    if new.is_signalling_export_show_dialog() {
        dialogs::sampleset_export(model_ptr.clone(), view, new.clone());
    }

    if new.is_signalling_export_begin_browse() {
        dialogs::choose_folder(
            model_ptr.clone(),
            view,
            SelectFolderDialogContext::BrowseForExportTargetDirectory,
            AppMessage::ExportTargetDirectoryBrowseSubmitted,
            AppMessage::ExportTargetDirectoryBrowseError,
        );
    }

    if old.are_add_fs_source_fields_valid() != new.are_add_fs_source_fields_valid() {
        view.sources_add_fs_add_button
            .set_sensitive(new.are_add_fs_source_fields_valid());
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

    if old.is_add_to_prev_set_enabled() != new.is_add_to_prev_set_enabled() {
        view.samples_sidebar_add_to_prev_button
            .set_visible(new.is_add_to_prev_set_enabled());
    }

    if old.set_most_recently_added_to() != new.set_most_recently_added_to() {
        if let Some(mru) = &new.set_most_recently_added_to() {
            if let Ok(set) = new.set(*mru) {
                view.samples_sidebar_add_to_prev_button
                    .set_label(&format!("Add to '{}'", set.name()));
            }
        }
    }

    if old.selected_set() != new.selected_set() {
        update_samplesets_detail(model_ptr.clone(), new.clone(), view);
    }

    if old.sets_map() != new.sets_map() {
        update_samplesets_list(model_ptr.clone(), new.clone(), view);
        update_samplesets_detail(model_ptr.clone(), new.clone(), view);

        if new.selected_sample().is_some() {
            update_samples_sidebar(model_ptr.clone(), new.clone(), view);
        }
    }

    if old.is_set_export_enabled() != new.is_set_export_enabled() {
        view.sets_details_export_button
            .set_sensitive(new.is_set_export_enabled());
    }

    if old.export_state() != new.export_state() {
        match new.export_state() {
            Some(ExportState::Exporting) => {
                if let Some(dv) = &new.export_dialog_view() {
                    dv.window.close();
                    view.progress_popup.set_visible(true);
                }
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

    if old.drum_machine_model() != new.drum_machine_model() {
        update_drum_machine_view(&new);
    }
}
