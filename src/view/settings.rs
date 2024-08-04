// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{glib::clone, prelude::*, StringList};

use crate::{
    config,
    ext::{OptionMapExt, WithModel},
    model::AppModelPtr,
    update,
    util::{set_dropdown_choice, strs_dropdown_get_selected},
    AppMessage,
};

use super::AsampoView;

pub fn setup_settings_page(model_ptr: AppModelPtr, view: &AsampoView) {
    view.settings_output_sample_rate_entry
        .set_model(Some(&StringList::new(
            &config::OUTPUT_SAMPLE_RATE_OPTIONS.keys(),
        )));

    view.settings_sample_rate_conversion_quality_entry
        .set_model(Some(&StringList::new(
            &config::SAMPLE_RATE_CONVERSION_QUALITY_OPTIONS.keys(),
        )));

    view.settings_sample_playback_behavior_entry
        .set_model(Some(&StringList::new(
            &config::SAMPLE_PLAYBACK_BEHAVIOR_OPTIONS.keys(),
        )));

    view.settings_save_on_quit_behavior_entry
        .set_model(Some(&StringList::new(
            &config::SAVE_ON_QUIT_BEHAVIOR_OPTIONS.keys(),
        )));

    view.settings_save_changed_sequence_behavior_entry
        .set_model(Some(&StringList::new(
            &config::SAVE_CHANGED_SEQUENCE_BEHAVIOR_OPTIONS.keys(),
        )));

    view.settings_save_changed_set_behavior_entry
        .set_model(Some(&StringList::new(
            &config::SAVE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS.keys(),
        )));

    view.settings_synchronize_changed_set_behavior_entry
        .set_model(Some(&StringList::new(
            &config::SYNCHRONIZE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS.keys(),
        )));

    // we don't want to trigger signals in setup_settings_page(), so update the settings
    // view before hooking up the signals.
    update_settings_page(model_ptr.clone(), view);

    view.settings_output_sample_rate_entry
        .connect_selected_item_notify(
            clone!(@strong model_ptr, @strong view => move |e: &gtk::DropDown| {
                update(model_ptr.clone(), &view, AppMessage::SettingsOutputSampleRateChanged(
                    strs_dropdown_get_selected(e)
                ))
            }),
        );

    view.settings_buffer_size_entry.connect_value_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::SpinButton| {
            update(
                model_ptr.clone(),
                &view,
                AppMessage::SettingsBufferSizeChanged(e.value() as u16)
            )
        }),
    );

    macro_rules! connect_changed {
        ($entry:ident, $message:ident) => {
            view.$entry
                .connect_selected_item_notify(
                    clone!(@strong model_ptr, @strong view => move |e: &gtk::DropDown| {
                        update(
                            model_ptr.clone(),
                            &view,
                            AppMessage::$message(strs_dropdown_get_selected(e))
                        )
                    }),
                );
        }
    }

    connect_changed!(
        settings_sample_rate_conversion_quality_entry,
        SettingsSampleRateConversionQualityChanged
    );

    connect_changed!(
        settings_sample_playback_behavior_entry,
        SettingsSamplePlaybackBehaviorChanged
    );

    connect_changed!(
        settings_save_on_quit_behavior_entry,
        SettingsSaveOnQuitBehaviorChanged
    );

    connect_changed!(
        settings_save_changed_sequence_behavior_entry,
        SettingsSaveChangedSequenceBehaviorChanged
    );

    connect_changed!(
        settings_save_changed_set_behavior_entry,
        SettingsSaveChangedSampleSetBehaviorChanged
    );

    connect_changed!(
        settings_synchronize_changed_set_behavior_entry,
        SettingsSynchronizeChangedSampleSetBehaviorChanged
    );
}

pub fn update_settings_page(model_ptr: AppModelPtr, view: &AsampoView) {
    model_ptr.with_model(|model| {
        let config = model.config();

        set_dropdown_choice(
            &view.settings_output_sample_rate_entry,
            &config::OUTPUT_SAMPLE_RATE_OPTIONS,
            &config.output_samplerate_hz,
        );

        view.settings_buffer_size_entry
            .set_value(config.buffer_size_frames.into());

        view.settings_latency_approx_label
            .set_text(model.latency_approx_label());

        set_dropdown_choice(
            &view.settings_sample_rate_conversion_quality_entry,
            &config::SAMPLE_RATE_CONVERSION_QUALITY_OPTIONS,
            &config.sample_rate_conversion_quality,
        );

        set_dropdown_choice(
            &view.settings_sample_playback_behavior_entry,
            &config::SAMPLE_PLAYBACK_BEHAVIOR_OPTIONS,
            &config.sample_playback_behavior,
        );

        set_dropdown_choice(
            &view.settings_save_on_quit_behavior_entry,
            &config::SAVE_ON_QUIT_BEHAVIOR_OPTIONS,
            &config.save_on_quit_behavior,
        );

        set_dropdown_choice(
            &view.settings_save_changed_sequence_behavior_entry,
            &config::SAVE_CHANGED_SEQUENCE_BEHAVIOR_OPTIONS,
            &config.save_changed_sequence_behavior,
        );

        set_dropdown_choice(
            &view.settings_save_changed_set_behavior_entry,
            &config::SAVE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS,
            &config.save_changed_set_behavior,
        );

        set_dropdown_choice(
            &view.settings_synchronize_changed_set_behavior_entry,
            &config::SYNCHRONIZE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS,
            &config.synchronize_changed_set_behavior,
        );

        if view.settings_config_save_path_entry.text() != config.config_save_path {
            view.settings_config_save_path_entry
                .set_text(&config.config_save_path);
        }

        model
    })
}
