// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::sync::{
    mpsc::{self, Sender},
    Arc, Mutex,
};

use anyhow::anyhow;
use libasampo::{
    samples::Sample,
    samplesets::{BaseSampleSet, DrumkitLabel, SampleSet, SampleSetOps},
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
    loaded_sampleset: Option<SampleSet>,
    sampleset: SampleSet,
    sources: Vec<Source>,
    activated_pad: usize,
    activated_part: usize,
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

        if self.activated_pad != other.activated_pad
            || self.activated_part != other.activated_part
            || self.sequence != other.sequence
            || self.loaded_sequence != other.loaded_sequence
            || self.sampleset != other.sampleset
            || self.loaded_sampleset != other.loaded_sampleset
        {
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
        Self {
            playback_state: PlaybackState::Stopped,
            waiting: false,
            render_thread_tx,
            event_rx: event_rx.map(|x| Arc::new(Mutex::new(x))),
            event_latest: None,
            loaded_sequence: None,
            sequence: Self::default_sequence(),
            loaded_sampleset: None,
            sampleset: SampleSet::BaseSampleSet(BaseSampleSet::new("Sampleset".to_string())),
            sources: Vec::new(),
            activated_pad: 8,
            activated_part: 0,
        }
    }

    pub fn new_with_render_thread(audiothread_tx: mpsc::Sender<audiothread::Message>) -> Self {
        let (render_tx, render_rx) = mpsc::channel::<drumkit_render_thread::Message>();
        let (event_rx, event_tx) = single_value_channel::channel::<DrumkitSequenceEvent>();

        let _ = drumkit_render_thread::spawn(audiothread_tx.clone(), render_rx, Some(event_tx));

        Self::new(Some(render_tx), Some(event_rx))
    }

    pub fn default_sequence() -> DrumkitSequence {
        let mut sequence =
            DrumkitSequence::new(TimeSpec::new(120, 4, 4).unwrap(), NoteLength::Sixteenth);
        sequence.set_len(16);

        sequence
    }

    pub fn is_equiv_default_sequence(seq: &DrumkitSequence) -> bool {
        let default = Self::default_sequence();

        let default_steps = (0..default.len())
            .map(|i| {
                default
                    .step(i)
                    .map(|info| info.triggers().clone())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        let seq_steps = (0..seq.len())
            .map(|i| {
                seq.step(i)
                    .map(|info| info.triggers().clone())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        !(default_steps != seq_steps
            || default.timespec() != seq.timespec()
            || default.step_base_len() != seq.step_base_len())
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

    pub fn set_activated_part(self, part: usize) -> AnyhowResult<DrumMachineModel> {
        if part < 4 {
            let needed_len = (part + 1) * 16;

            if needed_len > self.sequence.len() {
                let mut sequence = self.sequence.clone();
                sequence.set_len(needed_len);

                Ok(DrumMachineModel {
                    activated_part: part,
                    ..self.set_sequence(sequence, Mirroring::Mirror)?
                })
            } else {
                Ok(DrumMachineModel {
                    activated_part: part,
                    ..self
                })
            }
        } else {
            Err(anyhow!("Value out of range [0,3]"))
        }
    }

    pub fn truncate_parts_to(self, part: usize) -> AnyhowResult<DrumMachineModel> {
        if part < 4 {
            let needed_len = (part + 1) * 16;

            if needed_len < self.sequence.len() {
                let mut sequence = self.sequence.clone();
                sequence.set_len(needed_len);
                self.set_sequence(sequence, Mirroring::Mirror)
            } else {
                Ok(self)
            }
        } else {
            Err(anyhow!("Value out of range [0,3]"))
        }
    }

    pub fn activated_part(&self) -> usize {
        self.activated_part
    }

    pub fn load_sequence(self, sequence: DrumkitSequence) -> AnyhowResult<DrumMachineModel> {
        Ok(DrumMachineModel {
            loaded_sequence: Some(sequence.clone()),
            activated_part: 0,
            ..self.set_sequence(sequence, Mirroring::Mirror)?
        })
    }

    pub fn loaded_sequence(&self) -> Option<&DrumkitSequence> {
        self.loaded_sequence.as_ref()
    }

    pub fn clear_loaded_sequence(self) -> DrumMachineModel {
        DrumMachineModel {
            loaded_sequence: None,
            ..self
        }
    }

    pub fn is_sequence_modified(&self) -> bool {
        if let Some(seq) = self.loaded_sequence() {
            *seq != self.sequence
        } else {
            !Self::is_equiv_default_sequence(&self.sequence)
        }
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

        for i in 0..a.len() {
            if a.step(i).is_some() {
                assert!(b.step(i).is_some(), "Invalid swap");

                let astep = a.step(i).unwrap();
                let bstep = b.step(i).unwrap();

                assert_eq!(astep.triggers(), bstep.triggers(), "Invalid swap");
            } else {
                assert!(b.step(i).is_none(), "Invalid swap");
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
    pub fn swap_to_saved_sequence(self, saved_seq: DrumkitSequence) -> DrumMachineModel {
        Self::assert_valid_swap(&self.sequence, &saved_seq);

        DrumMachineModel {
            loaded_sequence: Some(saved_seq.clone()),
            sequence: saved_seq,
            ..self
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

    pub fn clear_sequence(self) -> AnyhowResult<DrumMachineModel> {
        Ok(DrumMachineModel {
            loaded_sequence: None,
            ..self.set_sequence(Self::default_sequence(), Mirroring::Mirror)?
        })
    }

    pub fn load_sampleset(
        self,
        sampleset: SampleSet,
        sources: Vec<Source>,
    ) -> AnyhowResult<DrumMachineModel> {
        Ok(DrumMachineModel {
            loaded_sampleset: Some(sampleset.clone()),
            ..self.set_sampleset(sampleset, sources, Mirroring::Mirror)?
        })
    }

    pub fn loaded_sampleset(&self) -> Option<&SampleSet> {
        self.loaded_sampleset.as_ref()
    }

    pub fn clear_loaded_sampleset(self) -> DrumMachineModel {
        DrumMachineModel {
            loaded_sampleset: None,
            ..self
        }
    }

    pub fn clear_sampleset(self) -> AnyhowResult<DrumMachineModel> {
        Ok(DrumMachineModel {
            loaded_sampleset: None,
            ..self.set_sampleset(
                SampleSet::BaseSampleSet(BaseSampleSet::new("Unnamed")),
                Vec::new(),
                Mirroring::Mirror,
            )?
        })
    }

    pub fn set_sampleset(
        self,
        sampleset: SampleSet,
        sources: Vec<Source>,
        mirroring: Mirroring,
    ) -> AnyhowResult<DrumMachineModel> {
        if mirroring == Mirroring::Mirror {
            self.render_thread_send(drumkit_render_thread::Message::LoadSampleSet(
                SampleSetSampleLoader::new(sampleset.clone(), sources.clone()),
            ))?;
        }

        Ok(DrumMachineModel {
            sampleset,
            sources,
            ..self
        })
    }

    pub fn sampleset(&self) -> &SampleSet {
        &self.sampleset
    }

    pub fn is_sampleset_modified(&self) -> bool {
        if let Some(set) = self.loaded_sampleset() {
            *set != self.sampleset
        } else {
            self.sampleset.len() > 0
        }
    }

    /// Reset the change-tracking for the loaded sampleset.
    ///
    /// Intended to be called after the sampleset has been saved externally, in order to
    /// clear any 'changed' status.
    pub fn commit_sampleset(self) -> AnyhowResult<DrumMachineModel> {
        if self.loaded_sampleset.is_some() {
            Ok(DrumMachineModel {
                loaded_sampleset: Some(self.sampleset.clone()),
                ..self
            })
        } else {
            Err(anyhow!("No sample set loaded"))
        }
    }

    fn assert_valid_sampleset_swap(a: &SampleSet, b: &SampleSet) -> bool {
        let list_a = a.list();
        let list_b = b.list();

        if list_a.len() != list_b.len() {
            return false;
        }

        for i in 0..list_a.len() {
            if list_a[i] != list_b[i]
                || a.get_label::<DrumkitLabel>(list_a[i]).ok()
                    != b.get_label::<DrumkitLabel>(list_b[i]).ok()
            {
                return false;
            }
        }

        true
    }

    pub fn swap_to_saved_sampleset(self, saved_set: SampleSet) -> DrumMachineModel {
        Self::assert_valid_sampleset_swap(&saved_set, &self.sampleset);

        DrumMachineModel {
            loaded_sampleset: Some(saved_set.clone()),
            sampleset: saved_set,
            ..self
        }
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
        let mut new_sampleset = self.sampleset.clone();

        // TODO: sampleset.remove_matching_label() in libasampo
        let mut samples_to_remove = Vec::<Sample>::new();

        for sample in new_sampleset.list() {
            let sample = sample.clone();

            if new_sampleset.get_label::<DrumkitLabel>(&sample).unwrap() == Some(label) {
                samples_to_remove.push(sample);
            }
        }

        for sample in samples_to_remove {
            new_sampleset.remove(&sample).unwrap();
        }

        new_sampleset.add(source, sample.clone())?;
        new_sampleset.set_label(&sample, label).unwrap();

        let new_sources = if !self.sources.contains(source) {
            self.sources.clone_and_push(source.clone())
        } else {
            self.sources.clone()
        };

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
