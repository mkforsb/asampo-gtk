// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{collections::HashMap, sync::mpsc, thread::JoinHandle};

use libasampo::{
    samplesets::{
        export::{self, ExportJob},
        SampleSet,
    },
    sources::Source,
};
use uuid::Uuid;

pub enum InputMessage {
    PerformExport(
        // TODO: implement Debug for libasampo::samplesets::export::DefaultIO
        ExportJob<export::DefaultIO>,
        HashMap<Uuid, Source>,
        SampleSet,
    ),
}

impl std::fmt::Debug for InputMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputMessage::PerformExport(_, _, _) => f.write_str("PerformExport"),
        }
    }
}

#[derive(Debug)]
pub enum OutputMessage {
    ExportError(anyhow::Error),
    ExportFinished,
}

pub fn spawn(rx: mpsc::Receiver<InputMessage>, tx: mpsc::Sender<OutputMessage>) -> JoinHandle<()> {
    std::thread::spawn(|| thread_main(rx, tx))
}

pub fn thread_main(rx: mpsc::Receiver<InputMessage>, tx: mpsc::Sender<OutputMessage>) {
    loop {
        match rx.recv() {
            Ok(message) => match message {
                InputMessage::PerformExport(mut job, sources, sampleset) => {
                    match job.perform(&sampleset, &sources) {
                        Ok(_) => tx.send(OutputMessage::ExportFinished).unwrap(),
                        Err(e) => tx.send(OutputMessage::ExportError(e.into())).unwrap(),
                    }
                }
            },

            Err(e) => {
                log::log!(log::Level::Error, "{e}");
                return;
            }
        }
    }
}
