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

type AnyhowResult<T> = Result<T, anyhow::Error>;

pub type RenderThreadTx = Sender<drumkit_render_thread::Message>;
pub type EventRx = Arc<Mutex<single_value_channel::Receiver<Option<DrumkitSequenceEvent>>>>;

#[derive(Clone, Debug)]
pub struct DrumMachineModel {
    pub render_thread_tx: Option<RenderThreadTx>,
    pub event_rx: Option<EventRx>,
    pub event_latest: Option<DrumkitSequenceEvent>,
    pub sequence: DrumkitSequence,
    pub activated_pad: usize,
}

impl PartialEq for DrumMachineModel {
    fn eq(&self, other: &Self) -> bool {
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

    pub fn activate_pad(self, pad: usize) -> AnyhowResult<DrumMachineModel> {
        if pad < 16 {
            Ok(DrumMachineModel {
                activated_pad: pad,
                ..self
            })
        } else {
            Err(anyhow!("Value out of range [0,15]"))
        }
    }

    pub fn sequence(&self) -> &DrumkitSequence {
        &self.sequence
    }
}
