// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::sync::{
    mpsc::{self, Sender},
    Arc, Mutex,
};

use anyhow::anyhow;
use libasampo::sequences::{
    drumkit_render_thread, DrumkitSequence, DrumkitSequenceEvent, NoteLength, StepSequenceOps,
    TimeSpec,
};

use crate::model::AnyhowResult;

pub type RenderThreadTx = Sender<drumkit_render_thread::Message>;
pub type EventRx = Arc<Mutex<single_value_channel::Receiver<Option<DrumkitSequenceEvent>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug)]
pub struct DrumMachineModel {
    playback_state: PlaybackState,
    waiting: bool,
    render_thread_tx: Option<RenderThreadTx>,
    event_rx: Option<EventRx>,
    event_latest: Option<DrumkitSequenceEvent>,
    sequence: DrumkitSequence,
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

        Self {
            playback_state: PlaybackState::Stopped,
            waiting: false,
            render_thread_tx,
            event_rx: event_rx.map(|x| Arc::new(Mutex::new(x))),
            event_latest: None,
            sequence: empty_sequence,
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

    pub fn set_sequence(self, sequence: DrumkitSequence) -> DrumMachineModel {
        DrumMachineModel { sequence, ..self }
    }

    pub fn sequence(&self) -> &DrumkitSequence {
        &self.sequence
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
}
