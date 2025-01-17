// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::HashMap;

use bolero::{check, gen, TypeGenerator};
use libasampo::{
    prelude::{SampleSetOps, StepSequenceOps},
    samples::{BaseSample, Sample, SampleMetadata, SampleURI},
    samplesets::{export::ExportJobMessage, BaseSampleSet, DrumkitLabel, SampleSet},
    sequences::{DrumkitSequence, NoteLength, TimeSpec},
    sources::{FakeSource, Source, SourceOps},
};

use crate::{
    bolero_utils::{Lcg, UuidGen},
    fake_audiohasher::FakeAudioHasher,
    labels::DRUM_LABELS,
};

// super = crate::model::core
use super::{arbitrary::CoreModelBuilderOps, ExportState};

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
fn test_add_insert_sequence_failure_uuid_in_use() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        lcg: Lcg,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let mut lcg = values.lcg.clone();

        if !model.sequences_list().is_empty() {
            let num_seqs = model.sequences_list().len();
            let seq_idx = lcg.next() % num_seqs;
            let seq_uuid = model.sequences_list()[seq_idx].uuid();

            let mut new_seq =
                DrumkitSequence::new(TimeSpec::new(120, 4, 4).unwrap(), NoteLength::Sixteenth);
            new_seq.set_uuid(seq_uuid);

            assert!(model.clone().add_sequence(new_seq.clone()).is_err());
            assert!(model.insert_sequence(new_seq, 0).is_err());
        }
    })
}

#[test]
fn test_add_insert_set_failure_uuid_in_use() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        lcg: Lcg,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let mut lcg = values.lcg.clone();

        if !model.sets_list().is_empty() {
            let num_sets = model.sets_list().len();
            let set_idx = lcg.next() % num_sets;
            let set_uuid = model.sets_list()[set_idx].uuid();

            let mut new_set = SampleSet::BaseSampleSet(BaseSampleSet::new_with_hasher::<
                FakeAudioHasher,
            >("set name"));
            new_set.set_uuid(set_uuid);

            assert!(model.clone().add_set(new_set.clone()).is_err());
            assert!(model.insert_set(new_set, 0).is_err());
        }
    })
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
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let new_source_uuid = values.uuid.get();

        if !model.sources_map().contains_key(&new_source_uuid) {
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
        }
    })
}

#[test]
fn test_add_to_set() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        lcg: Lcg,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();

        let has_samples = model
            .sources_list()
            .iter()
            .any(|s| !s.list().unwrap().is_empty());

        if has_samples && !model.sets_list().is_empty() {
            let mut lcg = values.lcg.clone();
            let num_sources = model.sources_list().len();
            let num_sets = model.sets_list().len();

            for _ in 0..100 {
                let source = model.sources_list()[lcg.next() % num_sources];
                let samples = source.list().unwrap();

                if samples.is_empty() {
                    continue;
                }

                let samples_idx = lcg.next() % samples.len();
                let set = model.sets_list()[lcg.next() % num_sets];

                if set.contains(&samples[samples_idx]) {
                    continue;
                }

                let updated_model = model
                    .clone()
                    .add_to_set(samples[samples_idx].clone(), set.uuid())
                    .unwrap();

                assert!(updated_model
                    .set(set.uuid())
                    .unwrap()
                    .contains(&samples[samples_idx]));

                break;
            }
        }
    })
}

#[test]
fn test_clear_sequences() {
    bolero_test!(|model| {
        let updated_model = model.clear_sequences();

        assert!(updated_model.sequences_map().is_empty());
        assert!(updated_model.sequences_list().is_empty());
    })
}

#[test]
fn test_clear_sets() {
    bolero_test!(|model| {
        let updated_model = model.clear_sets();

        assert!(updated_model.sets_map().is_empty());
        assert!(updated_model.sets_list().is_empty());
        assert_eq!(updated_model.selected_set(), None);
        assert_eq!(updated_model.set_most_recently_added_to(), None);
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
fn test_enable_disable_source_failure_uuid_not_present() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let bad_uuid = values.uuid.get();

        if !model.sources_map().contains_key(&bad_uuid) {
            assert!(model.clone().enable_source(bad_uuid).is_err());
            assert!(model.disable_source(bad_uuid).is_err());
        }
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
fn test_export_job_rx() {
    bolero_test!(|model| {
        let updated_model = model.set_export_job_rx(None);
        assert!(updated_model.export_job_rx().is_none());

        let (tx, rx) = std::sync::mpsc::channel::<ExportJobMessage>();

        let updated_model = updated_model.set_export_job_rx(Some(rx));
        let rx = updated_model.export_job_rx().unwrap();

        tx.send(ExportJobMessage::ItemsCompleted(123)).unwrap();

        assert!(match rx.recv() {
            Ok(message) => matches!(message, ExportJobMessage::ItemsCompleted(123)),
            Err(_) => false,
        })
    })
}

#[test]
fn test_export_state() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        lcg: Lcg,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let mut lcg = values.lcg.clone();
        let choice = lcg.next() % 3;

        if choice == 0 {
            let updated_model = model.set_export_state(None);
            assert_eq!(updated_model.export_state(), None);
        } else if choice == 1 {
            let updated_model = model.set_export_state(Some(ExportState::Finished));
            assert_eq!(updated_model.export_state(), Some(ExportState::Finished));
        } else {
            let updated_model = model.set_export_state(Some(ExportState::Exporting));
            assert_eq!(updated_model.export_state(), Some(ExportState::Exporting));
        }
    })
}

#[test]
fn test_get_or_create_set() {
    bolero_test!(|model| {
        let (updated_model, uuid) = model.clone().get_or_create_set("name of set").unwrap();
        assert_eq!(updated_model.set(uuid).unwrap().name(), "name of set");

        if !model.sets_list().is_empty() {
            let set = model.sets_list()[0].clone();

            if model
                .sets_list()
                .iter()
                .filter(|s| s.name() == set.name())
                .count()
                == 1
            {
                let (_, uuid) = model.get_or_create_set(set.name()).unwrap();
                assert_eq!(uuid, set.uuid());
            }
        }
    })
}

#[test]
fn test_insert_sequence() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
        insert_middle_pos: usize,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let uuid = values.uuid.get();

        if model.sequence(uuid).is_err() {
            let num_seqs_in_model = model.sequences_list().len();

            let mut seq_to_insert =
                DrumkitSequence::new(TimeSpec::new(120, 4, 4).unwrap(), NoteLength::Sixteenth);

            seq_to_insert.set_uuid(values.uuid.get());

            let inserted_start = model
                .clone()
                .insert_sequence(seq_to_insert.clone(), 0)
                .unwrap();

            let inserted_end = model
                .clone()
                .insert_sequence(seq_to_insert.clone(), num_seqs_in_model)
                .unwrap();

            assert_eq!(inserted_start.sequences_list().len(), num_seqs_in_model + 1);
            assert_eq!(inserted_start.sequences_list()[0].uuid(), uuid);

            assert_eq!(inserted_end.sequences_list().len(), num_seqs_in_model + 1);
            assert_eq!(
                inserted_end.sequences_list()[num_seqs_in_model].uuid(),
                uuid
            );

            if num_seqs_in_model > 0 {
                let insert_middle_pos = values.insert_middle_pos % num_seqs_in_model;

                let inserted_middle = model
                    .clone()
                    .insert_sequence(seq_to_insert.clone(), insert_middle_pos)
                    .unwrap();

                assert_eq!(
                    inserted_middle.sequences_list().len(),
                    num_seqs_in_model + 1
                );
                assert_eq!(
                    inserted_middle.sequences_list()[insert_middle_pos].uuid(),
                    uuid
                );
            }
        }
    })
}

#[test]
fn test_insert_set() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
        insert_middle_pos: usize,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let uuid = values.uuid.get();

        if model.set(uuid).is_err() {
            let num_sets_in_model = model.sets_list().len();

            let mut set_to_insert =
                BaseSampleSet::new_with_hasher::<FakeAudioHasher>("set to insert");
            set_to_insert.set_uuid(values.uuid.get());

            let inserted_start = model
                .clone()
                .insert_set(SampleSet::BaseSampleSet(set_to_insert.clone()), 0)
                .unwrap();

            let inserted_end = model
                .clone()
                .insert_set(
                    SampleSet::BaseSampleSet(set_to_insert.clone()),
                    num_sets_in_model,
                )
                .unwrap();

            assert_eq!(inserted_start.sets_list().len(), num_sets_in_model + 1);
            assert_eq!(inserted_start.sets_list()[0].uuid(), uuid);

            assert_eq!(inserted_end.sets_list().len(), num_sets_in_model + 1);
            assert_eq!(inserted_end.sets_list()[num_sets_in_model].uuid(), uuid);

            if num_sets_in_model > 0 {
                let insert_middle_pos = values.insert_middle_pos % num_sets_in_model;

                let inserted_middle = model
                    .clone()
                    .insert_set(
                        SampleSet::BaseSampleSet(set_to_insert.clone()),
                        insert_middle_pos,
                    )
                    .unwrap();

                assert_eq!(inserted_middle.sets_list().len(), num_sets_in_model + 1);
                assert_eq!(inserted_middle.sets_list()[insert_middle_pos].uuid(), uuid);
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
            .add_set(SampleSet::BaseSampleSet(BaseSampleSet::new_with_hasher::<
                FakeAudioHasher,
            >("test")))
            .unwrap();
        assert!(clone.is_modified_vs(&model));
    })
}

#[test]
fn test_is_modified_vs_added_source() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let uuid = values.uuid.get();

        if !model.sources_map().contains_key(&uuid) {
            let updated_model = model
                .clone()
                .add_source(Source::FakeSource(FakeSource {
                    name: None,
                    uri: "".to_string(),
                    uuid,
                    list: Vec::new(),
                    list_error: None,
                    stream: HashMap::new(),
                    stream_error: None,
                    enabled: true,
                }))
                .unwrap();

            assert!(updated_model.is_modified_vs(&model));
        }
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
                            .set_label(sample, Some(DrumkitLabel::Clap))
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
fn test_remove_sequence() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        lcg: Lcg,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let mut lcg = values.lcg.clone();

        if !model.sequences_list().is_empty() {
            let num_seqs = model.sequences_list().len();
            let seq_idx = lcg.next() % num_seqs;
            let seq_uuid = model.sequences_list()[seq_idx].uuid();

            let updated_model = model.remove_sequence(seq_uuid).unwrap();

            assert_eq!(updated_model.sequences_list().len(), num_seqs - 1);
            assert!(updated_model.sequence(seq_uuid).is_err());
        }
    })
}

#[test]
fn test_remove_source_loader_failure_uuid_not_present() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let uuid = values.uuid.get();

        if !model.sources_map().contains_key(&uuid) {
            assert!(model.remove_source_loader(uuid).is_err());
        }
    })
}

#[test]
fn test_selected_sequence() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        lcg: Lcg,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let mut lcg = values.lcg.clone();

        if !model.sequences_list().is_empty() {
            let num_seqs = model.sequences_list().len();
            let seq_idx = lcg.next() % num_seqs;
            let seq_uuid = model.sequences_list()[seq_idx].uuid();

            let updated_model = model.set_selected_sequence(Some(seq_uuid)).unwrap();
            assert_eq!(updated_model.selected_sequence(), Some(seq_uuid));

            let updated_model = updated_model.set_selected_sequence(None).unwrap();
            assert_eq!(updated_model.selected_sequence(), None);
        }
    })
}

#[test]
fn test_set_sample_label() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        lcg: Lcg,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let mut lcg = values.lcg.clone();

        if model.sets_list().is_empty() {
            return;
        }

        let num_sets = model.sets_list().len();
        let set_idx = lcg.next() % num_sets;

        if model.sets_list()[set_idx].list().is_empty() {
            return;
        }

        let set_uuid = model.sets_list()[set_idx].uuid();
        let num_samples = model.sets_list()[set_idx].list().len();
        let sample_idx = lcg.next() % num_samples;
        let sample = model.sets_list()[set_idx].list()[sample_idx].clone();
        let label_idx = lcg.next() % 17;

        let label = if label_idx == 0 {
            None
        } else {
            Some(DRUM_LABELS[label_idx - 1].1)
        };

        let updated_model = model.set_sample_label(set_uuid, &sample, label).unwrap();

        assert_eq!(
            updated_model.sets_list()[set_idx]
                .get_label::<DrumkitLabel>(&sample)
                .unwrap(),
            label
        );
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
fn test_set_selected_sequence_failure_uuid_not_present() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let uuid = values.uuid.get();

        if !model.sequences_map().contains_key(&uuid) {
            assert!(model.set_selected_sequence(Some(uuid)).is_err());
        }
    })
}

#[test]
fn test_set_selected_set() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        lcg: Lcg,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let mut lcg = values.lcg.clone();

        if !model.sets_list().is_empty() {
            let num_sets = model.sets_list().len();
            let set_idx = lcg.next() % num_sets;
            let set_uuid = model.sets_list()[set_idx].uuid();

            let updated_model = model.set_selected_set(Some(set_uuid)).unwrap();
            assert_eq!(updated_model.selected_set(), Some(set_uuid));

            let updated_model = updated_model.set_selected_set(None).unwrap();
            assert_eq!(updated_model.selected_set(), None);
        }
    })
}

#[test]
fn test_set_selected_set_failure_uuid_not_present() {
    #[derive(Debug, TypeGenerator)]
    struct Values {
        model_ops: Vec<CoreModelBuilderOps>,
        uuid: UuidGen,
    }

    bolero_test!(gen::<Values>(), |values| {
        let model = CoreModelBuilderOps::build_model(&values.model_ops).unwrap();
        let uuid = values.uuid.get();

        if !model.sets_map().contains_key(&uuid) {
            assert!(model.set_selected_set(Some(uuid)).is_err());
        }
    })
}

#[test]
fn test_sets_map_and_set() {
    bolero_test!(|model| {
        for uuid in model.sets_map().keys() {
            assert_eq!(*uuid, model.set(*uuid).unwrap().uuid());
        }
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
