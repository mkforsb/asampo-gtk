// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use crate::configfile::ConfigFile;

#[derive(Debug, Clone, PartialEq)]
pub enum SamplePlaybackBehavior {
    PlaySingleSample,
    PlayUntilEnd,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub output_samplerate_hz: u32,
    pub buffer_size_samples: u16,
    pub sample_rate_conversion_quality: audiothread::Quality,
    pub config_save_path: String,
    pub sample_playback_behavior: SamplePlaybackBehavior,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            output_samplerate_hz: 48000,
            buffer_size_samples: 1024,
            sample_rate_conversion_quality: audiothread::Quality::Lowest,
            config_save_path: ConfigFile::default_path(),
            sample_playback_behavior: SamplePlaybackBehavior::PlayUntilEnd,
        }
    }
}

impl AppConfig {
    pub fn fmt_latency_approx(&self) -> String {
        let samples = self.buffer_size_samples as f32;
        let rate = self.output_samplerate_hz as f32;

        format!("~{:.1} ms", (samples / rate) * 1000.0)
    }
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
