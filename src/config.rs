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
pub enum SaveWorkspaceBehavior {
    Ask,
    AskIfUnnamed,
    Save,
    DontSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveItemBehavior {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub output_samplerate_hz: u32,
    pub buffer_size_frames: u16,
    pub sample_rate_conversion_quality: audiothread::Quality,
    pub config_save_path: String,
    pub sample_playback_behavior: SamplePlaybackBehavior,
    pub save_workspace_behavior: SaveWorkspaceBehavior,
    pub save_changed_sequence_behavior: SaveItemBehavior,
    pub save_changed_set_behavior: SaveItemBehavior,
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
            save_workspace_behavior: SaveWorkspaceBehavior::Ask,
            save_changed_sequence_behavior: SaveItemBehavior::Ask,
            save_changed_set_behavior: SaveItemBehavior::Ask,
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

    update_with!(choice with_save_workspace_behavior_choice,
        save_workspace_behavior,
        SAVE_WORKSPACE_BEHAVIOR_OPTIONS,
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

pub const SAVE_WORKSPACE_BEHAVIOR_OPTIONS: [(&str, SaveWorkspaceBehavior); 4] = [
    ("Ask", SaveWorkspaceBehavior::Ask),
    (
        "Ask if unnamed, otherwise Save",
        SaveWorkspaceBehavior::AskIfUnnamed,
    ),
    ("Always Save", SaveWorkspaceBehavior::Save),
    ("Never Save", SaveWorkspaceBehavior::DontSave),
];

pub const SAVE_CHANGED_SEQUENCE_BEHAVIOR_OPTIONS: [(&str, SaveItemBehavior); 3] = [
    ("Ask", SaveItemBehavior::Ask),
    ("Always Save", SaveItemBehavior::Save),
    ("Always Discard", SaveItemBehavior::DontSave),
];

pub const SAVE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS: [(&str, SaveItemBehavior); 3] = [
    ("Ask", SaveItemBehavior::Ask),
    ("Always Save", SaveItemBehavior::Save),
    ("Always Discard", SaveItemBehavior::DontSave),
];

pub const SYNCHRONIZE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS: [(&str, SynchronizeBehavior); 3] = [
    ("Ask", SynchronizeBehavior::Ask),
    ("Always Synchronize", SynchronizeBehavior::Synchronize),
    ("Always Unlink", SynchronizeBehavior::Unlink),
];

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! appconf_test {
        (default $fn:ident, $field:ident $(,)?) => {{
            let conf = AppConfig {
                output_samplerate_hz: 44100,
                buffer_size_frames: 512,
                sample_rate_conversion_quality: audiothread::Quality::High,
                config_save_path: "abc123".to_string(),
                sample_playback_behavior: SamplePlaybackBehavior::PlaySingleSample,
                save_workspace_behavior: SaveWorkspaceBehavior::Save,
                save_changed_sequence_behavior: SaveItemBehavior::Save,
                save_changed_set_behavior: SaveItemBehavior::Save,
                synchronize_changed_set_behavior: SynchronizeBehavior::Synchronize,
            };

            assert_ne!(conf.$field, AppConfig::default().$field);

            let updated_conf = conf.$fn("invalid choice".to_string());
            assert_eq!(updated_conf.$field, AppConfig::default().$field);
        }};

        (choice $fn:ident, $field:ident, $choices:ident $(,)?) => {{
            let conf = AppConfig::default();

            for (key, val) in $choices.iter() {
                let updated_conf = conf.clone().$fn(String::from(*key));
                assert_eq!(updated_conf.$field, *val);
            }
        }};
    }

    #[test]
    fn test_fmt_latency_approx() {
        let conf1 = AppConfig::default().with_samplerate_choice("48 kHz".to_string());
        let conf2 = AppConfig::default().with_samplerate_choice("96 kHz".to_string());

        assert_eq!(
            conf1.clone().with_buffer_size(512).fmt_latency_approx(),
            "~10.7 ms"
        );
        assert_eq!(
            conf2.clone().with_buffer_size(512).fmt_latency_approx(),
            "~5.3 ms"
        );
        assert_eq!(
            conf1.clone().with_buffer_size(1024).fmt_latency_approx(),
            "~21.3 ms"
        );
        assert_eq!(
            conf2.clone().with_buffer_size(1024).fmt_latency_approx(),
            "~10.7 ms"
        );
    }

    #[test]
    fn test_default_fallbacks() {
        appconf_test!(default with_samplerate_choice, output_samplerate_hz);
        appconf_test!(default with_conversion_quality_choice, sample_rate_conversion_quality);
        appconf_test!(default with_sample_playback_behavior_choice, sample_playback_behavior);
        appconf_test!(default with_save_workspace_behavior_choice, save_workspace_behavior);

        appconf_test!(
            default with_save_changed_sequence_behavior_choice,
            save_changed_sequence_behavior,
        );

        appconf_test!(default with_save_changed_set_behavior_choice, save_changed_set_behavior);

        appconf_test!(
            default with_synchronize_changed_set_behavior_choice,
            synchronize_changed_set_behavior,
        );
    }

    #[test]
    fn test_choices() {
        appconf_test!(
            choice with_samplerate_choice,
            output_samplerate_hz,
            OUTPUT_SAMPLE_RATE_OPTIONS,
        );

        appconf_test!(
            choice with_conversion_quality_choice,
            sample_rate_conversion_quality,
            SAMPLE_RATE_CONVERSION_QUALITY_OPTIONS,
        );

        appconf_test!(
            choice with_sample_playback_behavior_choice,
            sample_playback_behavior,
            SAMPLE_PLAYBACK_BEHAVIOR_OPTIONS,
        );

        appconf_test!(
            choice with_save_workspace_behavior_choice,
            save_workspace_behavior,
            SAVE_WORKSPACE_BEHAVIOR_OPTIONS,
        );

        appconf_test!(
            choice with_save_changed_sequence_behavior_choice,
            save_changed_sequence_behavior,
            SAVE_CHANGED_SEQUENCE_BEHAVIOR_OPTIONS,
        );

        appconf_test!(
            choice with_save_changed_set_behavior_choice,
            save_changed_set_behavior,
            SAVE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS,
        );

        appconf_test!(
            choice with_synchronize_changed_set_behavior_choice,
            synchronize_changed_set_behavior,
            SYNCHRONIZE_CHANGED_SAMPLESET_BEHAVIOR_OPTIONS,
        );
    }
}
