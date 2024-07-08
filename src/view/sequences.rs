// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{
    glib::clone,
    prelude::{ButtonExt, FrameExt, WidgetExt},
};
use libasampo::samplesets::DrumkitLabel;

use crate::{
    model::{AppModel, DrumMachinePlaybackState},
    update, AppMessage, AppModelPtr, AsampoView,
};

pub const LABELS: [DrumkitLabel; 16] = [
    DrumkitLabel::RimShot,
    DrumkitLabel::Clap,
    DrumkitLabel::ClosedHihat,
    DrumkitLabel::OpenHihat,
    DrumkitLabel::CrashCymbal,
    DrumkitLabel::RideCymbal,
    DrumkitLabel::Shaker,
    DrumkitLabel::Perc1,
    DrumkitLabel::BassDrum,
    DrumkitLabel::SnareDrum,
    DrumkitLabel::LowTom,
    DrumkitLabel::MidTom,
    DrumkitLabel::HighTom,
    DrumkitLabel::Perc2,
    DrumkitLabel::Perc3,
    DrumkitLabel::Perc4,
];

pub fn setup_sequences_page(model_ptr: AppModelPtr, view: &AsampoView) {
    setup_drum_machine_view(model_ptr, view);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrumMachineView {
    play_button: gtk::Button,
    stop_button: gtk::Button,
    back_button: gtk::Button,
    pad_buttons: [gtk::Button; 16],
    part_buttons: [gtk::Button; 4],
    step_buttons: [gtk::Button; 16],
}

fn setup_drum_machine_view(model_ptr: AppModelPtr, view: &AsampoView) {
    let objects = gtk::Builder::from_resource("/drum-machine.ui");

    macro_rules! connect {
        (spinner $name:expr, $x:ident => $message:expr) => {
            objects.object::<gtk::SpinButton>($name).unwrap().connect_value_changed(
                clone!(@strong model_ptr, @strong view => move |$x: &gtk::SpinButton| {
                    update(model_ptr.clone(), &view, $message);
                })
            );
        };

        (button $name:expr, $message:expr) => {
            objects.object::<gtk::Button>($name).unwrap().connect_clicked(
                clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
                    update(model_ptr.clone(), &view, $message);
                })
            );
        };
    }

    connect!(spinner "sequences-editor-tempo-entry",
        x => AppMessage::DrumMachineTempoChanged(x.value_as_int() as u16));

    connect!(spinner "sequences-editor-swing-entry",
        x => AppMessage::DrumMachineSwingChanged(x.value_as_int() as u32));

    connect!(button "sequences-editor-play-button", AppMessage::DrumMachinePlayClicked);
    connect!(button "sequences-editor-stop-button", AppMessage::DrumMachineStopClicked);
    connect!(button "sequences-editor-back-button", AppMessage::DrumMachineBackClicked);
    connect!(button "sequences-editor-save-seq-button", AppMessage::DrumMachineSaveSequenceClicked);
    connect!(button "sequences-editor-save-seq-as-button",
        AppMessage::DrumMachineSaveSequenceAsClicked);
    connect!(button "sequences-editor-save-set-button",
        AppMessage::DrumMachineSaveSampleSetClicked);
    connect!(button "sequences-editor-save-set-as-button",
        AppMessage::DrumMachineSaveSampleSetAsClicked);

    let mut pad_buttons: Vec<gtk::Button> = vec![];
    let mut part_buttons: Vec<gtk::Button> = vec![];
    let mut step_buttons: Vec<gtk::Button> = vec![];

    for (index, _label) in LABELS.into_iter().enumerate() {
        connect!(button format!("sequences-editor-pad-{}", index),
            AppMessage::DrumMachinePadClicked(index));

        pad_buttons.push(
            objects
                .object::<gtk::Button>(format!("sequences-editor-pad-{}", index))
                .unwrap(),
        );
    }

    for index in 0..4 {
        connect!(button format!("sequences-editor-part-{}", index),
            AppMessage::DrumMachinePartClicked(index));

        part_buttons.push(
            objects
                .object::<gtk::Button>(format!("sequences-editor-part-{}", index))
                .unwrap(),
        );
    }

    for index in 0..16 {
        connect!(button format!("sequences-editor-step-{}", index),
            AppMessage::DrumMachineStepClicked(index));

        step_buttons.push(
            objects
                .object::<gtk::Button>(format!("sequences-editor-step-{}", index))
                .unwrap(),
        );
    }

    let pad_buttons: [gtk::Button; 16] = pad_buttons.try_into().unwrap();
    let part_buttons: [gtk::Button; 4] = part_buttons.try_into().unwrap();
    let step_buttons: [gtk::Button; 16] = step_buttons.try_into().unwrap();

    let mut model = model_ptr.take().unwrap();
    model = model.set_drum_machine_view(Some(DrumMachineView {
        play_button: objects
            .object::<gtk::Button>("sequences-editor-play-button")
            .unwrap(),
        stop_button: objects
            .object::<gtk::Button>("sequences-editor-stop-button")
            .unwrap(),
        back_button: objects
            .object::<gtk::Button>("sequences-editor-back-button")
            .unwrap(),
        pad_buttons,
        part_buttons,
        step_buttons,
    }));
    model_ptr.replace(Some(model));

    let root = objects.object::<gtk::Box>("drum-machine-root").unwrap();

    view.sequences_editor_drum_machine_frame
        .set_child(Some(&root));
}

pub fn update_drum_machine_view(model: AppModel) {
    let drum_machine_model = model.drum_machine_model();
    let drum_machine_view = model.drum_machine_view().unwrap();

    assert!(drum_machine_model.activated_pad() < 16);

    drum_machine_view.play_button.remove_css_class("activated");
    drum_machine_view.stop_button.remove_css_class("activated");
    drum_machine_view.back_button.remove_css_class("activated");

    match drum_machine_model.playback_state() {
        DrumMachinePlaybackState::Playing => {
            drum_machine_view.play_button.add_css_class("activated");
            drum_machine_view
                .play_button
                .set_icon_name("media-playback-start-symbolic");
        }
        DrumMachinePlaybackState::Paused => {
            drum_machine_view.play_button.add_css_class("activated");
            drum_machine_view
                .play_button
                .set_icon_name("media-playback-pause-symbolic");
        }
        DrumMachinePlaybackState::Stopped => {
            drum_machine_view.stop_button.add_css_class("activated");
            drum_machine_view
                .play_button
                .set_icon_name("media-playback-start-symbolic");
        }
    }

    if drum_machine_model.is_waiting()
        || drum_machine_model.playback_state() == DrumMachinePlaybackState::Stopped
    {
        for (i, _) in LABELS.iter().enumerate() {
            drum_machine_view.step_buttons[i].remove_css_class("playing");
        }
    } else {
        if let Some(event) = drum_machine_model.latest_event() {
            for (i, label) in LABELS.iter().enumerate() {
                if i == event.step {
                    drum_machine_view.step_buttons[i].add_css_class("playing");
                } else {
                    drum_machine_view.step_buttons[i].remove_css_class("playing");
                }

                if event.labels.contains(label) {
                    drum_machine_view.pad_buttons[i].add_css_class("playing");
                } else {
                    drum_machine_view.pad_buttons[i].remove_css_class("playing");
                }
            }
        }
    }

    for i in 0..16 {
        if i == drum_machine_model.activated_pad() {
            drum_machine_view.pad_buttons[i].add_css_class("activated");
        } else {
            drum_machine_view.pad_buttons[i].remove_css_class("activated");
        }
    }

    for i in 0..16 {
        if let Some(labels) = drum_machine_model.sequence().labels_at_step(i) {
            if labels.contains(&LABELS[drum_machine_model.activated_pad()]) {
                drum_machine_view.step_buttons[i].add_css_class("activated");
            } else {
                drum_machine_view.step_buttons[i].remove_css_class("activated");
            }
        }
    }
}
