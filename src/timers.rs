// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::sync::mpsc;

use gtk::glib::clone;
use libasampo::samplesets::export::ExportJobMessage;

use crate::{model::AppModelPtr, update, view::AsampoView, AppMessage};

/// Timer for AppMessage::TimerTick
pub fn init_timertick_timer(model_ptr: AppModelPtr, view: &AsampoView) {
    gtk::glib::timeout_add_seconds_local(
        1,
        clone!(@strong model_ptr, @strong view => move || {
            update(model_ptr.clone(), &view, AppMessage::TimerTick);
            gtk::glib::ControlFlow::Continue
        }),
    );
}

/// Timer for async/thread messaging
pub fn init_messaging_timer(model_ptr: AppModelPtr, view: &AsampoView) {
    gtk::glib::timeout_add_local(
        std::time::Duration::from_millis(50),
        clone!(@strong model_ptr, @strong view => move || {
            let model = model_ptr.take().unwrap();
            let export_job_rx = model.export_job_rx().clone();
            let sources_loaders = model.source_loaders().clone();
            model_ptr.set(Some(model));

            if let Some(rx) = export_job_rx {
                match rx.try_recv() {
                    Ok(message) => {
                        let mut messages = vec![message];
                        messages.extend(rx.try_iter());

                        let mut iter = messages.into_iter();
                        let mut message = iter.next();

                        loop {
                            if message.is_none() {
                                break;
                            }

                            if matches!(message, Some(ExportJobMessage::ItemsCompleted(_))) {
                                let mut proc_message = None;

                                while matches!(message, Some(ExportJobMessage::ItemsCompleted(_))) {
                                    proc_message = message;
                                    message = iter.next();
                                }

                                update(
                                    model_ptr.clone(),
                                    &view,
                                    AppMessage::ExportJobMessage(proc_message.unwrap())
                                );
                            } else {
                                update(
                                    model_ptr.clone(),
                                    &view,
                                    AppMessage::ExportJobMessage(message.unwrap())
                                );

                                message = iter.next();
                            }
                        }
                    }

                    Err(e) => {
                        match e {
                            mpsc::TryRecvError::Empty => (),
                            mpsc::TryRecvError::Disconnected =>
                                update(
                                    model_ptr.clone(),
                                    &view,
                                    AppMessage::ExportJobDisconnected
                                ),
                        }
                    },
                }
            }

            for uuid in sources_loaders.keys() {
                let recv = sources_loaders.get(uuid).unwrap();

                match recv.try_recv() {
                    Ok(message) => {
                        let mut messages = vec![message];
                        messages.extend(recv.try_iter());

                        update(
                            model_ptr.clone(),
                            &view,
                            AppMessage::SourceLoadingMessage(*uuid, messages)
                        );
                    }

                    Err(e) => {
                        match e {
                            mpsc::TryRecvError::Empty => (),
                            mpsc::TryRecvError::Disconnected => {
                                update(
                                    model_ptr.clone(),
                                    &view,
                                    AppMessage::SourceLoadingDisconnected(*uuid)
                                );
                            },
                        }
                    }
                };
            }

            gtk::glib::ControlFlow::Continue
        }),
    );
}

pub fn init_drum_machine_events_timer(model_ptr: AppModelPtr, view: &AsampoView) {
    gtk::glib::timeout_add_local(
        std::time::Duration::from_millis(4),
        clone!(@strong model_ptr, @strong view => move || {
            let model = model_ptr.take().unwrap();
            let event = model.drum_machine_poll_event();

            model_ptr.replace(Some(model));

            if let Some(ev) = event {
                update(
                    model_ptr.clone(),
                    &view,
                    AppMessage::DrumMachinePlaybackEvent(ev.clone())
                );
            }

            gtk::glib::ControlFlow::Continue
        }),
    );
}
