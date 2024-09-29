// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::HashMap;

use bolero::{check, gen};
use libasampo::{
    prelude::SampleSetOps,
    samplesets::{BaseSampleSet, DrumkitLabel, SampleSet},
    sequences::{DrumkitSequence, NoteLength, TimeSpec},
    sources::{FakeSource, Source, SourceOps},
};
use uuid::Uuid;

use super::arbitrary::CoreModelBuilderOps; // super = crate::model::core

macro_rules! bolero_test {
    ($fn:expr) => {{
        check!()
            .with_generator(gen::<Vec<CoreModelBuilderOps>>())
            .with_max_len(0)
            .with_iterations(1)
            .with_shrink_time(std::time::Duration::ZERO)
            .for_each(|ops| {
                CoreModelBuilderOps::build_model(ops).map($fn);
            });

        check!()
            .with_generator(gen::<Vec<CoreModelBuilderOps>>())
            .with_max_len(4294967296)
            // .with_iterations(10)
            .with_shrink_time(std::time::Duration::ZERO)
            .for_each(|ops| {
                CoreModelBuilderOps::build_model(ops).map($fn);
            });
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

// TODO: test_disable_source_samples_removed
// TODO: test_is_modified_vs_label_unassigned_in_set() (when generator generates labels)
// TODO: test_is_modified_vs_removed_sequence()
// TODO: test_is_modified_vs_changed_sequence_length()
// TODO: test_is_modified_vs_changed_sequence_tempo()
// TODO: test_is_modified_vs_changed_sequence_swing()
// TODO: test_is_modified_vs_changed_sequence_signature()
// TODO: test_is_modified_vs_changed_sequence_step()
