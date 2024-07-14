// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::sync::{
    mpsc::{self, Sender},
    Arc, Mutex,
};

use anyhow::anyhow;
use libasampo::{
    samples::{Sample, SampleOps},
    samplesets::{
        BaseSampleSet, ConcreteSampleSetLabelling, DrumkitLabel, DrumkitLabelling, SampleSet,
        SampleSetLabelling, SampleSetOps,
    },
    sequences::{
        drumkit_render_thread, DrumkitSequence, DrumkitSequenceEvent, NoteLength,
        SampleSetSampleLoader, StepSequenceOps, TimeSpec,
    },
    sources::Source,
};

use crate::{ext::ClonedVecExt, model::AnyhowResult};

pub type RenderThreadTx = Sender<drumkit_render_thread::Message>;
pub type EventRx = Arc<Mutex<single_value_channel::Receiver<Option<DrumkitSequenceEvent>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Mirror,
    Off,
}

#[derive(Clone, Debug)]
pub struct DrumMachineModel {
    playback_state: PlaybackState,
    waiting: bool,
    render_thread_tx: Option<RenderThreadTx>,
    event_rx: Option<EventRx>,
    event_latest: Option<DrumkitSequenceEvent>,
    loaded_sequence: Option<DrumkitSequence>,
    sequence: DrumkitSequence,
    sampleset: SampleSet,
    sources: Vec<Source>,
    activated_pad: usize,
}

impl PartialEq for DrumMachineModel {
    fn eq(&self, other: &Self) -> bool {
        if self.playback_state != other.playback_state {
            return false;
        }

        match (&self.event_latest, &other.event_latest) {
            (Some(a), Some(b)) => {
                if a.step != b.step || a.labels != b.labels {
                    return false;
                }
            }
            (None, None) => (),
            _ => return false,
        }

        if self.activated_pad != other.activated_pad || self.sequence != other.sequence {
            return false;
        }

        true
    }
}

impl DrumMachineModel {
    pub fn new(
        render_thread_tx: Option<Sender<drumkit_render_thread::Message>>,
        event_rx: Option<single_value_channel::Receiver<Option<DrumkitSequenceEvent>>>,
    ) -> Self {
        let mut empty_sequence =
            DrumkitSequence::new(TimeSpec::new(120, 4, 4).unwrap(), NoteLength::Sixteenth);
        empty_sequence.set_len(16);

        let mut empty_sampleset = BaseSampleSet::new("Sampleset".to_string());
        empty_sampleset.set_labelling(Some(SampleSetLabelling::DrumkitLabelling(
            DrumkitLabelling::new(),
        )));

        Self {
            playback_state: PlaybackState::Stopped,
            waiting: false,
            render_thread_tx,
            event_rx: event_rx.map(|x| Arc::new(Mutex::new(x))),
            event_latest: None,
            loaded_sequence: None,
            sequence: empty_sequence,
            sampleset: SampleSet::BaseSampleSet(empty_sampleset),
            sources: Vec::new(),
            activated_pad: 8,
        }
    }

    pub fn new_with_render_thread(audiothread_tx: mpsc::Sender<audiothread::Message>) -> Self {
        let (render_tx, render_rx) = mpsc::channel::<drumkit_render_thread::Message>();
        let (event_rx, event_tx) = single_value_channel::channel::<DrumkitSequenceEvent>();

        let _ = drumkit_render_thread::spawn(audiothread_tx.clone(), render_rx, Some(event_tx));

        Self::new(Some(render_tx), Some(event_rx))
    }

    pub fn is_render_thread_active(&self) -> bool {
        self.render_thread_tx.is_some()
    }

    pub fn render_thread_send(
        &self,
        message: drumkit_render_thread::Message,
    ) -> Result<(), anyhow::Error> {
        Ok(self
            .render_thread_tx
            .as_ref()
            .ok_or(anyhow!("Render thread not active"))?
            .send(message)?)
    }

    pub fn take_comms(self) -> (Option<RenderThreadTx>, Option<EventRx>) {
        (self.render_thread_tx, self.event_rx)
    }

    pub fn set_activated_pad(self, pad: usize) -> AnyhowResult<DrumMachineModel> {
        if pad < 16 {
            Ok(DrumMachineModel {
                activated_pad: pad,
                ..self
            })
        } else {
            Err(anyhow!("Value out of range [0,15]"))
        }
    }

    pub fn activated_pad(&self) -> usize {
        self.activated_pad
    }

    pub fn load_sequence(self, sequence: DrumkitSequence) -> AnyhowResult<DrumMachineModel> {
        Ok(DrumMachineModel {
            loaded_sequence: Some(sequence.clone()),
            ..self.set_sequence(sequence, Mirroring::Mirror)?
        })
    }

    pub fn loaded_sequence(&self) -> Option<&DrumkitSequence> {
        self.loaded_sequence.as_ref()
    }

    pub fn is_sequence_modified(&self) -> bool {
        self.loaded_sequence
            .as_ref()
            .is_some_and(|seq| *seq != self.sequence)
    }

    /// Reset the change-tracking for the loaded sequence.
    ///
    /// Intended to be called after the sequence has been saved externally, in order to
    /// clear any 'changed' status.
    pub fn commit_sequence(self) -> AnyhowResult<DrumMachineModel> {
        if self.loaded_sequence.is_some() {
            Ok(DrumMachineModel {
                loaded_sequence: Some(self.sequence.clone()),
                ..self
            })
        } else {
            Err(anyhow!("No sequence loaded"))
        }
    }

    fn assert_valid_swap(a: &DrumkitSequence, b: &DrumkitSequence) {
        assert_eq!(a.len(), b.len(), "Invalid swap");
        assert_eq!(a.timespec(), b.timespec(), "Invalid swap");
        assert_eq!(a.step_base_len(), a.step_base_len(), "Invalid swap");

        let sr = libasampo::sequences::Samplerate::try_from(44100).unwrap();

        for i in 0..a.len() {
            if a.step(i, sr).is_some() {
                assert!(b.step(i, sr).is_some(), "Invalid swap");

                let astep = a.step(i, sr).unwrap();
                let bstep = b.step(i, sr).unwrap();

                assert_eq!(astep.triggers(), bstep.triggers(), "Invalid swap");
            } else {
                assert!(b.step(i, sr).is_none(), "Invalid swap");
            }
        }
    }

    /// Swap out the drum machine sequence for an essentially identical sequence.
    ///
    /// Intended to be called after the sequence has been saved-as externally, in order
    /// to potentially update the name and UUID of the sequence (which may have changed
    /// due to the semantics of save-as), as well as to clear any 'changed' status.
    ///
    /// # Panics
    ///
    /// This method will panic if the given sequence differs from the current drum
    /// machine sequence in any way other than name and/or UUID.
    pub fn swap_to_saved_sequence(
        self,
        saved_seq: DrumkitSequence,
    ) -> AnyhowResult<DrumMachineModel> {
        if self.loaded_sequence.is_some() {
            Self::assert_valid_swap(&self.sequence, &saved_seq);

            Ok(DrumMachineModel {
                loaded_sequence: Some(saved_seq.clone()),
                sequence: saved_seq,
                ..self
            })
        } else {
            Err(anyhow!("No sequence loaded"))
        }
    }

    pub fn set_sequence(
        self,
        sequence: DrumkitSequence,
        mirroring: Mirroring,
    ) -> AnyhowResult<DrumMachineModel> {
        if mirroring == Mirroring::Mirror {
            self.render_thread_send(drumkit_render_thread::Message::SetSequence(
                sequence.clone(),
            ))?;
        }

        Ok(DrumMachineModel { sequence, ..self })
    }

    pub fn sequence(&self) -> &DrumkitSequence {
        &self.sequence
    }

    pub fn set_tempo(self, bpm: u16, mirroring: Mirroring) -> AnyhowResult<DrumMachineModel> {
        if mirroring == Mirroring::Mirror {
            self.render_thread_send(drumkit_render_thread::Message::SetTempo(bpm.try_into()?))?;
        }

        let mut sequence = self.sequence.clone();

        sequence.set_timespec(TimeSpec::new_with_swing(
            bpm,
            sequence.timespec().signature.upper(),
            sequence.timespec().signature.lower(),
            sequence.timespec().swing.get(),
        )?);

        Ok(DrumMachineModel { sequence, ..self })
    }

    pub fn set_swing(self, swing: f64, mirroring: Mirroring) -> AnyhowResult<DrumMachineModel> {
        if mirroring == Mirroring::Mirror {
            self.render_thread_send(drumkit_render_thread::Message::SetSwing(swing.try_into()?))?;
        }

        let mut sequence = self.sequence.clone();

        sequence.set_timespec(TimeSpec::new_with_swing(
            sequence.timespec().bpm.get(),
            sequence.timespec().signature.upper(),
            sequence.timespec().signature.lower(),
            swing,
        )?);

        Ok(DrumMachineModel { sequence, ..self })
    }

    pub fn set_latest_event(self, event: Option<DrumkitSequenceEvent>) -> DrumMachineModel {
        DrumMachineModel {
            event_latest: event,
            waiting: false,
            ..self
        }
    }

    pub fn latest_event(&self) -> Option<&DrumkitSequenceEvent> {
        self.event_latest.as_ref()
    }

    pub fn poll_event(&self) -> Option<DrumkitSequenceEvent> {
        if let Some(rx) = &self.event_rx {
            match rx.lock() {
                Ok(mut rx) => match rx.latest() {
                    Some(ev)
                        if self.event_latest.is_none()
                            || ev.step != self.event_latest.as_ref().unwrap().step =>
                    {
                        Some(ev.clone())
                    }
                    _ => None,
                },

                Err(e) => {
                    log::log!(log::Level::Warn, "Unable to lock event receiver: {e}");
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn play(self) -> AnyhowResult<DrumMachineModel> {
        self.render_thread_send(drumkit_render_thread::Message::Play)?;

        Ok(DrumMachineModel {
            playback_state: PlaybackState::Playing,
            waiting: true,
            ..self
        })
    }

    pub fn pause(self) -> AnyhowResult<DrumMachineModel> {
        self.render_thread_send(drumkit_render_thread::Message::Pause)?;

        Ok(DrumMachineModel {
            playback_state: PlaybackState::Paused,
            ..self
        })
    }

    pub fn stop(self) -> AnyhowResult<DrumMachineModel> {
        self.render_thread_send(drumkit_render_thread::Message::Stop)?;

        Ok(DrumMachineModel {
            playback_state: PlaybackState::Stopped,
            ..self
        })
    }

    pub fn rewind(&self) -> AnyhowResult<()> {
        self.render_thread_send(drumkit_render_thread::Message::ResetSequence)
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.playback_state
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting
    }

    pub fn assign_sample(
        self,
        source: &Source,
        sample: Sample,
        label: DrumkitLabel,
    ) -> AnyhowResult<DrumMachineModel> {
        let uri = sample.uri().clone();

        let mut new_sampleset = self.sampleset.clone();

        // TODO: make a better pattern for label types in libasampo
        // TODO: sampleset.remove_matching_label() in libasampo
        let mut samples_to_remove = Vec::<Sample>::new();

        if let Some(SampleSetLabelling::DrumkitLabelling(labelling)) = new_sampleset.labelling() {
            for sample in new_sampleset.list() {
                if *labelling.get(sample.uri()).unwrap() == label {
                    samples_to_remove.push(sample.clone());
                }
            }
        }

        for sample in samples_to_remove {
            new_sampleset.remove(&sample).unwrap();
        }

        new_sampleset.add(source, sample)?;

        let new_sources = if !self.sources.contains(source) {
            self.sources.clone_and_push(source.clone())
        } else {
            self.sources.clone()
        };

        match new_sampleset.labelling_mut() {
            Some(SampleSetLabelling::DrumkitLabelling(labelling)) => labelling.set(uri, label),
            _ => panic!("This should not be possible"),
        }

        self.render_thread_send(drumkit_render_thread::Message::LoadSampleSet(
            SampleSetSampleLoader::new(new_sampleset.clone(), new_sources.clone()),
        ))?;

        Ok(DrumMachineModel {
            sampleset: new_sampleset,
            sources: new_sources,
            ..self
        })
    }
}
