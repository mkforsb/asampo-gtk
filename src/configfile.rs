// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{io::Write, path::Path};

use serde::{Deserialize, Serialize};

use crate::config::{
    AppConfig, SamplePlaybackBehavior, SaveItemBehavior, SaveWorkspaceBehavior, SynchronizeBehavior,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioOutput {
    PulseAudioDefault,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(remote = "audiothread::Quality")]
pub enum QualitySerde {
    Lowest,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "crate::config::SaveWorkspaceBehavior")]
pub enum SaveWorkspaceBehaviorSerde {
    Ask,
    AskIfUnnamed,
    Save,
    DontSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "crate::config::SaveItemBehavior")]
pub enum SaveItemBehaviorSerde {
    Ask,
    Save,
    DontSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "crate::config::SynchronizeBehavior")]
pub enum SynchronizeBehaviorSerde {
    Ask,
    Synchronize,
    Unlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFileV3 {
    audio_output: AudioOutput,
    output_samplerate_hz: u32,
    buffer_size_samples: u16,

    #[serde(with = "QualitySerde")]
    sample_rate_conversion_quality: audiothread::Quality,

    config_save_path: String,

    #[serde(with = "PlaybackBehaviorSerde")]
    sample_playback_behavior: SamplePlaybackBehavior,

    #[serde(with = "SaveWorkspaceBehaviorSerde")]
    save_workspace_behavior: SaveWorkspaceBehavior,

    #[serde(with = "SaveItemBehaviorSerde")]
    save_changed_sequence_behavior: SaveItemBehavior,

    #[serde(with = "SaveItemBehaviorSerde")]
    save_changed_set_behavior: SaveItemBehavior,

    #[serde(with = "SynchronizeBehaviorSerde")]
    synchronize_changed_set_behavior: SynchronizeBehavior,
}

impl ConfigFileV3 {
    pub fn into_appconfig(self) -> AppConfig {
        AppConfig {
            output_samplerate_hz: self.output_samplerate_hz,
            buffer_size_frames: self.buffer_size_samples,
            sample_rate_conversion_quality: self.sample_rate_conversion_quality,
            config_save_path: self.config_save_path,
            sample_playback_behavior: self.sample_playback_behavior,
            save_workspace_behavior: self.save_workspace_behavior,
            save_changed_sequence_behavior: self.save_changed_sequence_behavior,
            save_changed_set_behavior: self.save_changed_set_behavior,
            synchronize_changed_set_behavior: self.synchronize_changed_set_behavior,
        }
    }

    pub fn from_appconfig(config: &AppConfig) -> ConfigFileV3 {
        ConfigFileV3 {
            audio_output: AudioOutput::PulseAudioDefault,
            output_samplerate_hz: config.output_samplerate_hz,
            buffer_size_samples: config.buffer_size_frames,
            sample_rate_conversion_quality: config.sample_rate_conversion_quality,
            config_save_path: config.config_save_path.clone(),
            sample_playback_behavior: config.sample_playback_behavior,
            save_workspace_behavior: config.save_workspace_behavior,
            save_changed_sequence_behavior: config.save_changed_sequence_behavior,
            save_changed_set_behavior: config.save_changed_set_behavior,
            synchronize_changed_set_behavior: config.synchronize_changed_set_behavior,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "SaveBehavior")]
pub enum SaveBehaviorV2 {
    Ask,
    Save,
    DontSave,
}

impl From<SaveBehaviorV2> for SaveWorkspaceBehavior {
    fn from(value: SaveBehaviorV2) -> Self {
        match value {
            SaveBehaviorV2::Ask => SaveWorkspaceBehavior::Ask,
            SaveBehaviorV2::Save => SaveWorkspaceBehavior::Save,
            SaveBehaviorV2::DontSave => SaveWorkspaceBehavior::DontSave,
        }
    }
}

impl From<SaveBehaviorV2> for SaveItemBehavior {
    fn from(value: SaveBehaviorV2) -> Self {
        match value {
            SaveBehaviorV2::Ask => SaveItemBehavior::Ask,
            SaveBehaviorV2::Save => SaveItemBehavior::Save,
            SaveBehaviorV2::DontSave => SaveItemBehavior::DontSave,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFileV2 {
    audio_output: AudioOutput,
    output_samplerate_hz: u32,
    buffer_size_samples: u16,

    #[serde(with = "QualitySerde")]
    sample_rate_conversion_quality: audiothread::Quality,

    config_save_path: String,

    #[serde(with = "PlaybackBehaviorSerde")]
    sample_playback_behavior: SamplePlaybackBehavior,

    save_on_quit_behavior: SaveBehaviorV2,
    save_changed_sequence_behavior: SaveBehaviorV2,
    save_changed_set_behavior: SaveBehaviorV2,

    #[serde(with = "SynchronizeBehaviorSerde")]
    synchronize_changed_set_behavior: SynchronizeBehavior,
}

impl ConfigFileV2 {
    pub fn into_appconfig(self) -> AppConfig {
        AppConfig {
            output_samplerate_hz: self.output_samplerate_hz,
            buffer_size_frames: self.buffer_size_samples,
            sample_rate_conversion_quality: self.sample_rate_conversion_quality,
            config_save_path: self.config_save_path,
            sample_playback_behavior: self.sample_playback_behavior,
            save_workspace_behavior: self.save_on_quit_behavior.into(),
            save_changed_sequence_behavior: self.save_changed_sequence_behavior.into(),
            save_changed_set_behavior: self.save_changed_set_behavior.into(),
            synchronize_changed_set_behavior: self.synchronize_changed_set_behavior,
        }
    }

    // pub fn from_appconfig(config: &AppConfig) -> ConfigFileV2 {
    //     ConfigFileV2 {
    //         audio_output: AudioOutput::PulseAudioDefault,
    //         output_samplerate_hz: config.output_samplerate_hz,
    //         buffer_size_samples: config.buffer_size_frames,
    //         sample_rate_conversion_quality: config.sample_rate_conversion_quality,
    //         config_save_path: config.config_save_path.clone(),
    //         sample_playback_behavior: config.sample_playback_behavior,
    //         save_on_quit_behavior: config.save_on_quit_behavior,
    //         save_changed_sequence_behavior: config.save_changed_sequence_behavior,
    //         save_changed_set_behavior: config.save_changed_set_behavior,
    //         synchronize_changed_set_behavior: config.synchronize_changed_set_behavior,
    //     }
    // }
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
            buffer_size_frames: self.buffer_size_samples,
            sample_rate_conversion_quality: self.sample_rate_conversion_quality,
            config_save_path: self.config_save_path,
            sample_playback_behavior: self.sample_playback_behavior,
            save_workspace_behavior: Into::into(SaveBehaviorV2::Ask),
            save_changed_sequence_behavior: Into::into(SaveBehaviorV2::Ask),
            save_changed_set_behavior: Into::into(SaveBehaviorV2::Ask),
            synchronize_changed_set_behavior: SynchronizeBehavior::Ask,
        }
    }

    // pub fn from_appconfig(config: &AppConfig) -> ConfigFileV1 {
    //     ConfigFileV1 {
    //         audio_output: AudioOutput::PulseAudioDefault,
    //         output_samplerate_hz: config.output_samplerate_hz,
    //         buffer_size_samples: config.buffer_size_frames,
    //         sample_rate_conversion_quality: config.sample_rate_conversion_quality,
    //         config_save_path: config.config_save_path.clone(),
    //         sample_playback_behavior: config.sample_playback_behavior,
    //     }
    // }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigFile {
    V1(ConfigFileV1),
    V2(ConfigFileV2),
    V3(ConfigFileV3),
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
        let json = serde_json::to_string(&ConfigFile::V3(ConfigFileV3::from_appconfig(config)))?;

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

            ConfigFile::V2(conf) => Ok(AppConfig {
                config_save_path: filename.to_string(),
                ..conf.into_appconfig()
            }),

            ConfigFile::V3(conf) => Ok(AppConfig {
                config_save_path: filename.to_string(),
                ..conf.into_appconfig()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use uuid::Uuid;

    use super::*;

    fn asset_path(name: &str) -> String {
        format!(
            "{}/test_assets/configfile/{name}",
            env::var("CARGO_MANIFEST_DIR").unwrap()
        )
    }

    #[test]
    fn test_load_v1() {
        let conf = ConfigFile::load(asset_path("v1.json").as_str()).unwrap();

        assert_eq!(conf.output_samplerate_hz, 96000);
        assert_eq!(conf.buffer_size_frames, 2048);
        assert_eq!(
            conf.sample_rate_conversion_quality,
            audiothread::Quality::Low
        );
        assert_eq!(conf.config_save_path, asset_path("v1.json"));
        assert_eq!(
            conf.sample_playback_behavior,
            SamplePlaybackBehavior::PlayUntilEnd
        );
    }

    #[test]
    fn test_load_v2() {
        let conf = ConfigFile::load(asset_path("v2.json").as_str()).unwrap();

        assert_eq!(conf.output_samplerate_hz, 44100);
        assert_eq!(conf.buffer_size_frames, 1024);
        assert_eq!(
            conf.sample_rate_conversion_quality,
            audiothread::Quality::High
        );
        assert_eq!(conf.config_save_path, asset_path("v2.json"));
        assert_eq!(
            conf.sample_playback_behavior,
            SamplePlaybackBehavior::PlayUntilEnd
        );
        assert_eq!(conf.save_workspace_behavior, SaveWorkspaceBehavior::Ask);
        assert_eq!(
            conf.save_changed_sequence_behavior,
            SaveItemBehavior::DontSave
        );
        assert_eq!(conf.save_changed_set_behavior, SaveItemBehavior::Ask);
        assert_eq!(
            conf.synchronize_changed_set_behavior,
            SynchronizeBehavior::Synchronize
        );
    }

    #[test]
    fn test_load_v3() {
        let conf = ConfigFile::load(asset_path("v3.json").as_str()).unwrap();

        assert_eq!(conf.output_samplerate_hz, 48000);
        assert_eq!(conf.buffer_size_frames, 768);
        assert_eq!(
            conf.sample_rate_conversion_quality,
            audiothread::Quality::Lowest
        );
        assert_eq!(conf.config_save_path, asset_path("v3.json"));
        assert_eq!(
            conf.sample_playback_behavior,
            SamplePlaybackBehavior::PlaySingleSample
        );
        assert_eq!(
            conf.save_workspace_behavior,
            SaveWorkspaceBehavior::AskIfUnnamed
        );
        assert_eq!(conf.save_changed_sequence_behavior, SaveItemBehavior::Ask);
        assert_eq!(conf.save_changed_set_behavior, SaveItemBehavior::Save);
        assert_eq!(
            conf.synchronize_changed_set_behavior,
            SynchronizeBehavior::Unlink
        );
    }

    #[test]
    fn test_save() {
        if let Ok(exists) = fs::exists("/tmp") {
            if exists {
                let mut conf = AppConfig {
                    buffer_size_frames: 12345,
                    ..AppConfig::default()
                };

                conf.config_save_path = format!("/tmp/{}.json", Uuid::new_v4());
                ConfigFile::save(&conf, &conf.config_save_path).unwrap();

                let conf2 = ConfigFile::load(&conf.config_save_path).unwrap();

                assert_eq!(conf, conf2);
            }
        }
    }
}
