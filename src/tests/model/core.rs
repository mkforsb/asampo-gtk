// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::HashMap;

use bolero::{check, gen, TypeGenerator};
use libasampo::{
    prelude::SampleSetOps,
    samples::{BaseSample, Sample, SampleMetadata, SampleURI},
    samplesets::{BaseSampleSet, DrumkitLabel, SampleSet},
    sequences::{DrumkitSequence, NoteLength, TimeSpec},
    sources::{FakeSource, Source, SourceOps},
};
use uuid::Uuid;

use super::arbitrary::{CoreModelBuilderOps, UuidGen}; // super = crate::model::core

macro_rules! bolero_test {
    ($fn:expr) => {{
        bolero_test!(gen::<Vec<CoreModelBuilderOps>>(), |ops| {
            CoreModelBuilderOps::build_model(ops).map($fn)
        })
    }};

    ($gen:expr, $each:expr) => {{
        check!()
            .with_generator($gen)
            .with_max_len(0)
            .with_iterations(1)
            .with_shrink_time(std::time::Duration::ZERO)
            .for_each($each);

        check!()
            .with_generator($gen)
            .with_max_len(4294967296)
            // .with_iterations(10)
            .with_shrink_time(std::time::Duration::ZERO)
            .for_each($each);
    }};
}

#[test]
fn test_add_source_failure_uuid_exists() {
    bolero_test!(|model| {
        for (uuid, _) in model.sources_map().iter() {
            assert!(model
                .clone()
                .add_source(Source::FakeSource(FakeSource {
                    name: None,
                    uri: "".to_string(),
                    uuid: *uuid,
                    list: Vec::new(),
                    list_error: None,
                    stream: HashMap::new(),
                    stream_error: None,
                    enabled: true,
                }))
                .is_err())
        }
    })
}

#[test]
fn test_add_source_loader_failure_uuid_exists() {
    #[derive(Debug, TypeGenerator)]
    struct ModelAndUuid {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
    }

    bolero_test!(gen::<ModelAndUuid>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();

        if !model.source_loaders().contains_key(&values.uuid.get()) {
            let (_tx, rx) = std::sync::mpsc::channel();
            let updated_model = model.add_source_loader(values.uuid.get(), rx).unwrap();

            let (_tx, rx) = std::sync::mpsc::channel();
            let updated_model = updated_model.add_source_loader(values.uuid.get(), rx);

            assert!(updated_model.is_err());
        }
    })
}

#[test]
fn test_add_source_success() {
    bolero_test!(|model| {
        let mut new_source_uuid = Uuid::new_v4();
        let mut attempts = 0;

        while attempts < 1000 && model.sources_map().contains_key(&new_source_uuid) {
            attempts += 1;
            new_source_uuid = Uuid::new_v4();
        }

        assert!(model
            .add_source(Source::FakeSource(FakeSource {
                name: None,
                uri: "".to_string(),
                uuid: new_source_uuid,
                list: Vec::new(),
                list_error: None,
                stream: HashMap::new(),
                stream_error: None,
                enabled: true,
            }))
            .is_ok())
    })
}

#[test]
fn test_clear_sources() {
    bolero_test!(|model| {
        if !model.sources_map().is_empty() {
            let updated_model = model.clear_sources();

            assert_eq!(updated_model.sources_map().len(), 0);
            assert_eq!(updated_model.sources_list().len(), 0);
            assert_eq!(updated_model.samples().len(), 0);
            assert!(!updated_model.has_sources_loading());
        }
    })
}

#[test]
fn test_disable_source_failure_uuid_not_present() {
    bolero_test!(|model| {
        let source_uuids = model
            .sources_map()
            .iter()
            .map(|(uuid, _)| uuid)
            .cloned()
            .collect::<Vec<_>>();

        let mut bad_uuid = Uuid::new_v4();
        let mut attempts = 0;

        while attempts < 1000 && source_uuids.contains(&bad_uuid) {
            attempts += 1;
            bad_uuid = Uuid::new_v4();
        }

        assert!(model.disable_source(bad_uuid).is_err());
    })
}

#[test]
fn test_disable_source_samples_removed() {
    bolero_test!(|model| {
        if !model.sources_list().is_empty() {
            let mut updated_model = model.clone();

            for source in model
                .sources_list()
                .iter()
                .filter(|source| source.is_enabled())
            {
                let num_samples_pre = updated_model.samples().len();
                let num_samples_src = source.list().unwrap().len();

                updated_model = updated_model.disable_source(*source.uuid()).unwrap();

                assert_eq!(
                    num_samples_src,
                    num_samples_pre - updated_model.samples().len()
                );

                for sample in source.list().unwrap().iter() {
                    assert!(!updated_model.samples().contains(sample));
                }
            }
        }
    })
}

#[test]
fn test_enable_source_failure_uuid_not_present() {
    bolero_test!(|model| {
        let source_uuids = model
            .sources_map()
            .iter()
            .map(|(uuid, _)| uuid)
            .cloned()
            .collect::<Vec<_>>();

        let mut bad_uuid = Uuid::new_v4();
        let mut attempts = 0;

        while attempts < 1000 && source_uuids.contains(&bad_uuid) {
            attempts += 1;
            bad_uuid = Uuid::new_v4();
        }

        assert!(model.enable_source(bad_uuid).is_err());
    })
}

#[test]
fn test_enable_source_samples_loaded() {
    bolero_test!(|model| {
        let mut model = model;
        let source_uuids = model
            .sources_map()
            .iter()
            .map(|(uuid, _)| uuid)
            .cloned()
            .collect::<Vec<_>>();

        for uuid in source_uuids.iter() {
            model = model.disable_source(*uuid).unwrap();
        }

        assert!(model.samples.borrow().is_empty());

        for uuid in source_uuids.iter() {
            let result = model.enable_source(*uuid);
            assert!(result.is_ok());

            model = result.unwrap();
            assert!(model.has_sources_loading());

            let loaders = model.source_loaders().iter().collect::<Vec<_>>();

            for (_, loader) in loaders.iter() {
                while let Ok(msg) = loader.recv() {
                    model.handle_source_loader(vec![msg])
                }
            }

            for sample in model.source(*uuid).unwrap().list().unwrap() {
                assert!(model.samples().contains(&sample));
            }
        }
    })
}

#[test]
fn test_is_modified_vs_added_sequence() {
    bolero_test!(|model| {
        let mut clone = model.clone();
        clone = clone
            .add_sequence(DrumkitSequence::new(
                TimeSpec::new(120, 4, 4).unwrap(),
                NoteLength::Sixteenth,
            ))
            .unwrap();
        assert!(clone.is_modified_vs(&model));
    })
}

#[test]
fn test_is_modified_vs_added_set() {
    bolero_test!(|model| {
        let mut clone = model.clone();
        clone = clone
            .add_set(SampleSet::BaseSampleSet(BaseSampleSet::new("test")))
            .unwrap();
        assert!(clone.is_modified_vs(&model));
    })
}

#[test]
fn test_is_modified_vs_added_source() {
    bolero_test!(|model| {
        let mut clone = model.clone();
        clone = clone
            .add_source(Source::FakeSource(FakeSource {
                name: None,
                uri: "".to_string(),
                uuid: Uuid::new_v4(),
                list: Vec::new(),
                list_error: None,
                stream: HashMap::new(),
                stream_error: None,
                enabled: true,
            }))
            .unwrap();

        assert!(clone.is_modified_vs(&model));
    })
}

#[test]
fn test_is_modified_vs_changed_sequences_order() {
    bolero_test!(|model| {
        if model.sequences.len() > 2 {
            let mut clone = model.clone();
            clone.sequences_order.reverse();
            assert!(clone.is_modified_vs(&model));
        }
    })
}

#[test]
fn test_is_modified_vs_changed_sets_order() {
    bolero_test!(|model| {
        if model.sets.len() > 2 {
            let mut clone = model.clone();
            clone.sets_order.reverse();
            assert!(clone.is_modified_vs(&model));
        }
    })
}

#[test]
fn test_is_modified_vs_changed_sources_order() {
    bolero_test!(|model| {
        if model.sources.len() > 2 {
            let mut clone = model.clone();
            clone.sources_order.reverse();
            assert!(clone.is_modified_vs(&model));
        }
    })
}

#[test]
fn test_is_modified_vs_enabled_disabled_source() {
    bolero_test!(|model| {
        if let Some(enabled_source) = model.sources_list().iter().find(|s| s.is_enabled()) {
            let updated_model = model
                .clone()
                .disable_source(*enabled_source.uuid())
                .unwrap();
            assert!(updated_model.is_modified_vs(&model));
        }

        if let Some(disabled_source) = model.sources_list().iter().find(|s| !s.is_enabled()) {
            let updated_model = model
                .clone()
                .enable_source(*disabled_source.uuid())
                .unwrap();
            assert!(updated_model.is_modified_vs(&model));
        }
    })
}

#[test]
fn test_is_modified_vs_label_assigned_in_set() {
    bolero_test!(|model| {
        model
            .sets_list()
            .iter()
            .find(|set| set.len() > 0)
            .map(|set| {
                set.list()
                    .iter()
                    .find(|sample| {
                        set.get_label::<DrumkitLabel>(sample)
                            .is_ok_and(|x| x.is_none())
                    })
                    .map(|sample| {
                        let mut updated_model = model.clone();
                        let updated_set = updated_model.set_mut(set.uuid()).unwrap();
                        updated_set
                            .set_label::<DrumkitLabel, _>(sample, Some(DrumkitLabel::Clap))
                            .unwrap();
                        assert!(updated_model.is_modified_vs(&model));
                    })
            })
    })
}

#[test]
fn test_is_modified_vs_removed_sample_set() {
    bolero_test!(|model| {
        if !model.sets.is_empty() {
            for uuid in [
                *model.sets_order.first().unwrap(),
                *model.sets_order.last().unwrap(),
                model.sets_order[model.sets_order.len() / 2],
            ]
            .iter()
            {
                let updated_model = model.clone().remove_set(*uuid).unwrap();
                assert!(updated_model.is_modified_vs(&model));
            }
        }
    })
}

#[test]
fn test_is_modified_vs_removed_source() {
    bolero_test!(|model| {
        if !model.sources.is_empty() {
            let uuid = *model.sources_map().keys().next().unwrap();
            let updated_model = model.clone().remove_source(uuid).unwrap();
            assert!(updated_model.is_modified_vs(&model));
        }
    })
}

#[test]
fn test_is_modified_vs_sample_added_to_set() {
    bolero_test!(|model| {
        model
            .samples()
            .iter()
            .find_map(|sample| {
                model
                    .sets_list()
                    .iter()
                    .find(|set| !set.contains(sample))
                    .map(|set| (sample.clone(), (*set).clone()))
            })
            .map(|(sample, set)| {
                let mut updated_model = model.clone();
                let updated_set = updated_model.set_mut(set.uuid()).unwrap();
                updated_set.add_with_hash(sample.clone(), "hash".to_string());
                assert!(updated_model.is_modified_vs(&model));
            })
    })
}

#[test]
fn test_is_modified_vs_sample_removed_from_set() {
    bolero_test!(|model| {
        model
            .sets_list()
            .iter()
            .find(|set| set.len() > 0)
            .map(|set| {
                for sample in [
                    set.list().first().cloned().unwrap(),
                    set.list().last().cloned().unwrap(),
                    set.list()[set.list().len() / 2],
                ]
                .iter()
                {
                    let mut updated_model = model.clone();
                    let updated_set = updated_model.set_mut(set.uuid()).unwrap();
                    updated_set.remove(sample).unwrap();
                    assert!(updated_model.is_modified_vs(&model));

                    let updated_model = model.clone().remove_from_set(sample, set.uuid()).unwrap();
                    assert!(updated_model.is_modified_vs(&model));
                }
            })
    })
}

#[test]
fn test_is_modified_vs_self() {
    bolero_test!(|model| {
        assert!(!model.is_modified_vs(&model));
    })
}

#[test]
fn test_set_selected_sample() {
    bolero_test!(|model| {
        let sample = Sample::BaseSample(BaseSample::new(
            SampleURI::new("test".to_string()),
            "test".to_string(),
            SampleMetadata {
                rate: 44100,
                channels: 2,
                src_fmt_display: "PCM".to_string(),
                size_bytes: None,
                length_millis: None,
            },
            None,
        ));

        let sample2 = Sample::BaseSample(BaseSample::new(
            SampleURI::new("test2".to_string()),
            "test2".to_string(),
            SampleMetadata {
                rate: 44100,
                channels: 2,
                src_fmt_display: "PCM".to_string(),
                size_bytes: None,
                length_millis: None,
            },
            None,
        ));

        let updated_model = model.set_selected_sample(Some(sample.clone()));
        assert_eq!(updated_model.selected_sample(), Some(sample).as_ref());
        assert_ne!(updated_model.selected_sample(), Some(sample2).as_ref());
    })
}

#[test]
fn test_source() {
    bolero_test!(|model| {
        for (uuid, _) in model.sources_map().iter() {
            assert!(model.source(*uuid).is_ok());
        }
    })
}

#[test]
fn test_sources_map_and_sources_list() {
    bolero_test!(|model| {
        assert_eq!(model.sources_map().len(), model.sources_list().len());

        assert!(model
            .sources_map()
            .iter()
            .all(|(_, val)| model.sources_list().contains(&val)));

        assert!(model.sources_list().iter().all(|listval| model
            .sources_map()
            .iter()
            .any(|(_, mapval)| *listval == mapval)));
    })
}

// TODO: test_is_modified_vs_label_unassigned_in_set() (when generator generates labels)
// TODO: test_is_modified_vs_removed_sequence()
// TODO: test_is_modified_vs_changed_sequence_length()
// TODO: test_is_modified_vs_changed_sequence_tempo()
// TODO: test_is_modified_vs_changed_sequence_swing()
// TODO: test_is_modified_vs_changed_sequence_signature()
// TODO: test_is_modified_vs_changed_sequence_step()
