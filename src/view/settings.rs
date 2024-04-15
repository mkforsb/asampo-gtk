// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{
    glib::{clone, Object},
    prelude::*,
    StringObject,
};

use crate::{config::AppConfig, ext::*, model::AppModelPtr, update, AppMessage};

use super::AsampoView;

pub fn setup_settings_page(model_ptr: AppModelPtr, view: &AsampoView) {
    // we don't want to trigger signals in setup_settings_page(), so update the settings
    // view before hooking up the signals.
    update_settings_page(model_ptr.clone(), view);

    view.settings_output_sample_rate_entry
        .connect_selected_item_notify(
            clone!(@strong model_ptr, @strong view => move |e: &gtk::DropDown| {
                update(model_ptr.clone(), &view, AppMessage::SettingsOutputSampleRateChanged(
                    strs_dropdown_map_selected(e, |s| match s {
                        "44100 Hz" => 44100,
                        "48000 Hz" => 48000,
                        "96000 Hz" => 96000,
                        "192000 Hz" => 192000,
                        _ => {
                            log::log!(log::Level::Error, "Unknown sample rate setting");
                            AppConfig::default().output_samplerate_hz
                        }
                    })
                ))
            }),
        );

    view.settings_buffer_size_entry.connect_value_changed(
        clone!(@strong model_ptr, @strong view => move |e: &gtk::SpinButton| {
            update(model_ptr.clone(), &view, AppMessage::SettingsBufferSizeChanged(e.value() as u16))
        })
    );
}

pub fn update_settings_page(model_ptr: AppModelPtr, view: &AsampoView) {
    model_ptr.with_model(|model| {
        let config = model.config.as_ref().expect("A config should be present");

        let rate_fmt = format!("{} Hz", config.output_samplerate_hz);

        if let Some(rate_idx) = view
            .settings_output_sample_rate_entry
            .model()
            .expect("Sample rate dropdown should have a model")
            .iter()
            .position(|x: Result<Object, _>| {
                x.expect("ListModel should not be mutated while iterating")
                    .dynamic_cast_ref::<StringObject>()
                    .expect("ListModel should contain StringObject items")
                    .string()
                    == rate_fmt
            })
        {
            view.settings_output_sample_rate_entry
                .set_selected(rate_idx.try_into().unwrap());
        }

        view.settings_buffer_size_entry
            .set_value(config.buffer_size_samples.into());

        view.settings_latency_approx_label
            .set_text(&model.values.settings_latency_approx_label);

        let conv_fmt = match config.sample_rate_conversion_quality {
            audiothread::Quality::Fastest => String::from("Fastest"),
            audiothread::Quality::Low => String::from("Low"),
            audiothread::Quality::Medium => String::from("Medium"),
            audiothread::Quality::High => String::from("High"),
        };

        if let Some(conv_idx) = view
            .settings_sample_rate_conversion_quality_entry
            .model()
            .expect("Conversion quality dropdown should have a model")
            .iter()
            .position(|x: Result<Object, _>| {
                x.expect("ListModel should not be mutated while iterating")
                    .dynamic_cast_ref::<StringObject>()
                    .expect("ListModel should contain StringObject items")
                    .string()
                    == conv_fmt
            })
        {
            view.settings_sample_rate_conversion_quality_entry
                .set_selected(conv_idx.try_into().unwrap());
        }

        if view.settings_config_save_path_entry.text() != config.config_save_path {
            view.settings_config_save_path_entry
                .set_text(&config.config_save_path);
        }

        model
    })
}

fn strs_dropdown_get_selected(e: &gtk::DropDown) -> String {
    e.model()
        .expect("Dropdown should have a model")
        .item(e.selected())
        .expect("Selected item should be obtainable from model")
        .dynamic_cast_ref::<StringObject>()
        .expect("ListModel should contain StringObject items")
        .string()
        .to_string()
}

fn strs_dropdown_map_selected<T>(e: &gtk::DropDown, f: fn(&str) -> T) -> T {
    f(&strs_dropdown_get_selected(e))
}
