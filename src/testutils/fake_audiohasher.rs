// MIT License
//
// Copyright (c) 2025 Mikael Forsberg (github.com/mkforsb)

use libasampo::{audiohash::AudioHasher, errors::Error as LibasampoError, sources::SourceReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeAudioHasher;

impl AudioHasher for FakeAudioHasher {
    // TODO: add self-parameter in libasampo trait and use a field FakeAudioHasher.hashvalue
    fn audio_hash(_reader: SourceReader) -> Result<String, LibasampoError> {
        Ok(String::from("fake hash"))
    }
}
