// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{self, Sender},
};

use libasampo::sequences::{
    drumkit_render_thread, DrumkitSequence, DrumkitSequenceEvent, NoteLength, StepSequenceOps,
    TimeSpec,
};

#[derive(Clone, Debug)]
pub struct DrumMachineModel {
    pub render_thread_tx: Option<Sender<drumkit_render_thread::Message>>,
    pub event_rx: Option<Rc<RefCell<single_value_channel::Receiver<Option<DrumkitSequenceEvent>>>>>,
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
            event_rx: event_rx.map(|x| Rc::new(RefCell::new(x))),
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
}
