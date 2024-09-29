// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::HashSet;

use bolero::{check, gen, TypeGenerator};
use libasampo::{
    samples::{BaseSample, SampleMetadata, SampleURI},
    sequences::{NoteLength, TimeSpec},
    sources::FakeSource,
};

use crate::labels::DRUM_LABELS;

use super::*; // super = crate::model::core

#[derive(Debug, Clone, TypeGenerator)]
pub struct UuidGen {
    val: u128,
}

#[derive(Debug, Clone, TypeGenerator)]
pub struct SampleGen {
    name_uri_uuidgen: UuidGen,
}

#[derive(Debug, Clone, TypeGenerator)]
pub enum NoteLengthGen {
    Eighth,
    Sixteenth,
}

#[derive(Debug, Clone, TypeGenerator)]
#[allow(clippy::enum_variant_names)]
pub enum CoreModelBuilderOps {
    AddSource {
        name_uri_uuidgen: UuidGen,
        uuidgen: UuidGen,
        list_and_stream: Vec<(SampleGen, Vec<f32>)>,
        enabled: bool,
    },
    AddSet {
        uuidgen: UuidGen,
        name_uuidgen: UuidGen,
        members: Vec<(usize, Option<usize>)>,
    },
    AddSequence {
        uuidgen: UuidGen,

        #[generator(gen::<usize>().with().bounds(0..=64))]
        steps: usize,

        #[generator(gen::<u16>().with().bounds(1..=300))]
        bpm: u16,

        #[generator(gen::<u8>().with().bounds(0..=100))]
        swing: u8,

        #[generator(gen::<u8>().with().bounds(2..=12))]
        sig_upper: u8,

        #[generator(gen::<u8>().with().bounds(2..=12))]
        sig_lower: u8,

        base_note_len: NoteLengthGen,
    },
}

impl CoreModelBuilderOps {
    pub fn build_model(ops: &[CoreModelBuilderOps]) -> Option<CoreModel> {
        fn uuidstr(val: u128) -> String {
            Uuid::from_u128(val).to_string()
        }

        let mut model = CoreModel::new();
        let mut samples = Vec::new();

        for op in ops.iter().cloned() {
            match op {
                Self::AddSource {
                    name_uri_uuidgen,
                    uuidgen,
                    list_and_stream,
                    enabled,
                } => {
                    let name = uuidstr(name_uri_uuidgen.val);
                    let uri = uuidstr(name_uri_uuidgen.val);
                    let uuid = Uuid::from_u128(uuidgen.val);

                    let list: Vec<Sample> = list_and_stream
                        .iter()
                        .map(|(sample, data)| {
                            Sample::BaseSample(BaseSample::new(
                                SampleURI::new(uuidstr(sample.name_uri_uuidgen.val)),
                                uuidstr(sample.name_uri_uuidgen.val),
                                SampleMetadata {
                                    rate: 44100,
                                    channels: 2,
                                    src_fmt_display: "PCM".to_string(),
                                    size_bytes: Some(data.len() as u64),
                                    length_millis: None,
                                },
                                Some(uuid),
                            ))
                        })
                        .collect();

                    samples.extend(list.iter().cloned());

                    let stream = list_and_stream
                        .iter()
                        .map(|(sample, data)| {
                            (
                                SampleURI::new(uuidstr(sample.name_uri_uuidgen.val)),
                                data.clone(),
                            )
                        })
                        .collect();

                    model = match model.add_source(Source::FakeSource(FakeSource {
                        name: Some(name),
                        uri,
                        uuid,
                        list,
                        list_error: None,
                        stream,
                        stream_error: None,
                        enabled,
                    })) {
                        Ok(updated_model) => {
                            if enabled {
                                let loaders =
                                    updated_model.source_loaders().iter().collect::<Vec<_>>();

                                for (_, loader) in loaders.iter() {
                                    while let Ok(msg) = loader.recv() {
                                        updated_model.handle_source_loader(vec![msg])
                                    }
                                }
                            }

                            updated_model
                        }
                        Err(_) => {
                            // eprintln!("failed to add source: {e:?}");
                            return None;
                        }
                    }
                }

                Self::AddSet { .. } if samples.is_empty() => (),

                Self::AddSet {
                    uuidgen,
                    name_uuidgen,
                    members,
                } => {
                    let mut set = BaseSampleSet::new(uuidstr(name_uuidgen.val));
                    set.set_uuid(Uuid::from_u128(uuidgen.val));

                    let mut labels_avail: HashSet<usize> = (0..=15).collect();

                    members.iter().for_each(|(index, _)| {
                        set.add_with_hash(
                            samples[index % samples.len()].clone(),
                            "hash".to_string(),
                        );
                    });

                    members.iter().for_each(|(index, label)| {
                        if (16 - labels_avail.len()) < set.len()
                            && label.is_some_and(|lb| labels_avail.contains(&lb))
                        {
                            labels_avail.remove(&label.unwrap());

                            set.set_label::<DrumkitLabel, _>(
                                &samples[index % samples.len()],
                                Some(DRUM_LABELS[label.unwrap() % 16].1),
                            )
                            .unwrap();
                        }
                    });

                    assert!((16 - labels_avail.len()) <= set.len());

                    model = match model.add_set(SampleSet::BaseSampleSet(set)) {
                        Ok(updated_model) => updated_model,
                        Err(_) => {
                            // eprintln!("failed to add set: {e:?}");
                            return None;
                        }
                    };
                }

                Self::AddSequence {
                    uuidgen,
                    steps,
                    bpm,
                    swing,
                    sig_upper,
                    sig_lower,
                    base_note_len,
                } => {
                    let mut sequence = DrumkitSequence::new(
                        TimeSpec::new_with_swing(
                            bpm,
                            sig_upper,
                            sig_lower,
                            swing as f64 / 100.0f64,
                        )
                        .ok()?,
                        match base_note_len {
                            NoteLengthGen::Eighth => NoteLength::Eighth,
                            NoteLengthGen::Sixteenth => NoteLength::Sixteenth,
                        },
                    );

                    sequence.set_uuid(Uuid::from_u128(uuidgen.val));
                    sequence.set_len(steps);

                    model = match model.add_sequence(sequence) {
                        Ok(updated_model) => updated_model,
                        Err(_) => {
                            // eprintln!("failed to add sequence: {e:?}");
                            return None;
                        }
                    };
                }
            }
        }

        Some(model)
    }
}

#[test]
fn test_core_model_builder() {
    check!()
        .with_generator(gen::<Vec<CoreModelBuilderOps>>())
        .with_max_len(999999999)
        .for_each(|ops| {
            if let Some(model) = CoreModelBuilderOps::build_model(ops) {
                println!(
                    "model with {} sources, {} sets, {} sequences",
                    model.sources.len(),
                    model.sets.len(),
                    model.sequences.len(),
                );
            }
        });
}