// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{io::Write, path::Path};

use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, SamplePlaybackBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioOutput {
    PulseAudioDefault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(remote = "audiothread::Quality")]
pub enum QualitySerde {
    Fastest,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(remote = "crate::config::SamplePlaybackBehavior")]
pub enum PlaybackBehaviorSerde {
    PlaySingleSample,
    PlayUntilEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFileV1 {
    audio_output: AudioOutput,
    output_samplerate_hz: u32,
    buffer_size_samples: u16,

    #[serde(with = "QualitySerde")]
    sample_rate_conversion_quality: audiothread::Quality,

    config_save_path: String,

    #[serde(with = "PlaybackBehaviorSerde")]
    sample_playback_behavior: SamplePlaybackBehavior,
}

impl ConfigFileV1 {
    pub fn into_appconfig(self) -> AppConfig {
        AppConfig {
            output_samplerate_hz: self.output_samplerate_hz,
            buffer_size_samples: self.buffer_size_samples,
            sample_rate_conversion_quality: self.sample_rate_conversion_quality.clone(),
            config_save_path: self.config_save_path,
            sample_playback_behavior: self.sample_playback_behavior,
        }
    }

    pub fn from_appconfig(config: &AppConfig) -> ConfigFileV1 {
        ConfigFileV1 {
            audio_output: AudioOutput::PulseAudioDefault,
            output_samplerate_hz: config.output_samplerate_hz,
            buffer_size_samples: config.buffer_size_samples,
            sample_rate_conversion_quality: config.sample_rate_conversion_quality.clone(),
            config_save_path: config.config_save_path.clone(),
            sample_playback_behavior: config.sample_playback_behavior.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigFile {
    V1(ConfigFileV1),
}

impl ConfigFile {
    pub fn default_path() -> String {
        dirs::config_dir()
            .expect("System should have a common config dir")
            .join("asampo")
            .join("asampo.conf")
            .to_str()
            .expect("Should be able to construct the default config path")
            .to_string()
    }

    pub fn save(config: &AppConfig, filename: &str) -> Result<(), anyhow::Error> {
        let json = serde_json::to_string(&ConfigFile::V1(ConfigFileV1::from_appconfig(config)))?;

        {
            if let Some(path) = Path::new(filename).parent() {
                std::fs::create_dir_all(path)?;
            }

            let mut fd = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(filename)?;

            fd.write_all(json.as_bytes())?;
        }

        Ok(())
    }

    pub fn load(filename: &str) -> Result<AppConfig, anyhow::Error> {
        match serde_json::from_str::<ConfigFile>(&String::from_utf8(std::fs::read(filename)?)?)? {
            ConfigFile::V1(conf) => Ok(AppConfig {
                config_save_path: filename.to_string(),
                ..conf.into_appconfig()
            }),
        }
    }
}
