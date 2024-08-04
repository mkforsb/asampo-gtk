// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use crate::{configfile::ConfigFile, ext::OptionMapExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplePlaybackBehavior {
    PlaySingleSample,
    PlayUntilEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveBehavior {
    Ask,
    Save,
    DontSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynchronizeBehavior {
    Ask,
    Synchronize,
    Unlink,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub output_samplerate_hz: u32,
    pub buffer_size_frames: u16,
    pub sample_rate_conversion_quality: audiothread::Quality,
    pub config_save_path: String,
    pub sample_playback_behavior: SamplePlaybackBehavior,
    pub save_on_quit_behavior: SaveBehavior,
    pub save_changed_sequence_behavior: SaveBehavior,
    pub save_changed_set_behavior: SaveBehavior,
    pub synchronize_changed_set_behavior: SynchronizeBehavior,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            output_samplerate_hz: 48000,
            buffer_size_frames: 1024,
            sample_rate_conversion_quality: audiothread::Quality::Lowest,
            config_save_path: ConfigFile::default_path(),
            sample_playback_behavior: SamplePlaybackBehavior::PlayUntilEnd,
            save_on_quit_behavior: SaveBehavior::Ask,
            save_changed_sequence_behavior: SaveBehavior::Ask,
            save_changed_set_behavior: SaveBehavior::Ask,
            synchronize_changed_set_behavior: SynchronizeBehavior::Ask,
        }
    }
}

macro_rules! update_with {
    (plain $fname:ident, $field:ident, $typ:ty) => {
        pub fn $fname(self, $field: $typ) -> AppConfig {
            AppConfig { $field, ..self }
        }
    };

    (choice $fname:ident, $field:ident, $options:ident, $descr:expr) => {
        pub fn $fname(self, choice: String) -> AppConfig {
            AppConfig {
                $field: match $options.value_for(&choice) {
                    Some(value) => value.clone(),
                    None => {
                        log::log!(
                            log::Level::Error,
                            "Unknown output {} setting, using default",
                            $descr,
                        );
                        AppConfig::default().$field
                    }
                },
                ..self
            }
        }
    };
}

impl AppConfig {
    pub fn fmt_latency_approx(&self) -> String {
        let samples = self.buffer_size_frames as f32;
        let rate = self.output_samplerate_hz as f32;

        format!("~{:.1} ms", (samples / rate) * 1000.0)
    }

    // update_with!(plain with_sample_rate, output_samplerate_hz, u32);

    update_with!(choice with_samplerate_choice,
        output_samplerate_hz, OUTPUT_SAMPLE_RATE_OPTIONS, "sample rate");

    update_with!(plain with_buffer_size, buffer_size_frames, u16);

    // update_with!(plain with_conversion_quality,
    //     sample_rate_conversion_quality,
    //     audiothread::Quality);

    update_with!(choice with_conversion_quality_choice,
        sample_rate_conversion_quality,
        SAMPLE_RATE_CONVERSION_QUALITY_OPTIONS,
        "conversion quality");

    // update_with!(plain with_save_path, config_save_path, String);

    // update_with!(plain with_sample_playback_behavior,
    //     sample_playback_behavior,
    //     SamplePlaybackBehavior);

    update_with!(choice with_sample_playback_behavior_choice,
        sample_playback_behavior,
        SAMPLE_PLAYBACK_BEHAVIOR_OPTIONS,
        "sample playback behavior");

    update_with!(choice with_save_on_quit_behavior_choice,
        save_on_quit_behavior,
        SAVE_ON_QUIT_BEHAVIOR_OPTIONS,
        "save on quit behavior");

    update_with!(choice with_save_changed_sequence_behavior_choice,
        save_changed_sequence_behavior,
        SAVE_CHANGED_SEQUENCE_BEHAVIOR_OPTIONS,
        "save changed sequence behavior");

    update_with!(choice with_save_changed_set_behavior_choice,
        save_changed_set_behavior,
        SAVE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS,
        "save changed set behavior");

    update_with!(choice with_synchronize_changed_set_behavior_choice,
        synchronize_changed_set_behavior,
        SYNCHRONIZE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS,
        "synchronize changed set behavior");
}

pub const OUTPUT_SAMPLE_RATE_OPTIONS: [(&str, u32); 4] = [
    ("44.1 kHz", 44100),
    ("48 kHz", 48000),
    ("96 kHz", 96000),
    ("192 kHz", 192000),
];

pub const SAMPLE_RATE_CONVERSION_QUALITY_OPTIONS: [(&str, audiothread::Quality); 4] = [
    ("Lowest", audiothread::Quality::Lowest),
    ("Low", audiothread::Quality::Low),
    ("Medium", audiothread::Quality::Medium),
    ("High", audiothread::Quality::High),
];

pub const SAMPLE_PLAYBACK_BEHAVIOR_OPTIONS: [(&str, SamplePlaybackBehavior); 2] = [
    (
        "Play only most recently selected sample",
        SamplePlaybackBehavior::PlaySingleSample,
    ),
    (
        "Let each sample play to completion",
        SamplePlaybackBehavior::PlayUntilEnd,
    ),
];

pub const SAVE_ON_QUIT_BEHAVIOR_OPTIONS: [(&str, SaveBehavior); 3] = [
    ("Ask", SaveBehavior::Ask),
    ("Always Save", SaveBehavior::Save),
    ("Never Save", SaveBehavior::DontSave),
];

pub const SAVE_CHANGED_SEQUENCE_BEHAVIOR_OPTIONS: [(&str, SaveBehavior); 3] = [
    ("Ask", SaveBehavior::Ask),
    ("Always Save", SaveBehavior::Save),
    ("Always Discard", SaveBehavior::DontSave),
];

pub const SAVE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS: [(&str, SaveBehavior); 3] = [
    ("Ask", SaveBehavior::Ask),
    ("Always Save", SaveBehavior::Save),
    ("Always Discard", SaveBehavior::DontSave),
];

pub const SYNCHRONIZE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS: [(&str, SynchronizeBehavior); 3] = [
    ("Ask", SynchronizeBehavior::Ask),
    ("Always Synchronize", SynchronizeBehavior::Synchronize),
    ("Always Unlink", SynchronizeBehavior::Unlink),
];
