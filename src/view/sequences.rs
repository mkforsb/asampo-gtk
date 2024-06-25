// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{
    glib::clone,
    prelude::{ButtonExt, FrameExt},
};
use libasampo::samplesets::DrumkitLabel;

use crate::{update, AppMessage, AppModelPtr, AsampoView};

pub fn setup_sequences_page(model_ptr: AppModelPtr, view: &AsampoView) {
    setup_drum_machine(model_ptr, view);
}

// #[derive(Debug, Clone)]
// pub struct DrumMachineView {
//
// }

fn setup_drum_machine(model_ptr: AppModelPtr, view: &AsampoView) {
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
        x => AppMessage::DrumMachineTempoChanged(x.value_as_int() as u32));

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

    let labels = vec![
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

    for (index, label) in labels.into_iter().enumerate() {
        connect!(button format!("sequences-editor-pad-{}", index),
            AppMessage::DrumMachinePadClicked(label));
    }

    for index in 0..4 {
        connect!(button format!("sequences-editor-part-{}", index),
            AppMessage::DrumMachinePartClicked(index));
    }

    for index in 0..16 {
        connect!(button format!("sequences-editor-step-{}", index),
            AppMessage::DrumMachineStepClicked(index));
    }

    let root = objects.object::<gtk::Box>("drum-machine-root").unwrap();

    view.sequences_editor_drum_machine_frame
        .set_child(Some(&root));
}
