// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::{
    glib::clone,
    prelude::{ButtonExt, EventControllerExt, FrameExt, ListBoxRowExt, WidgetExt},
    Button, EventControllerKey, GestureClick, SpinButton,
};
use libasampo::{samplesets::DrumkitLabel, sequences::StepSequenceOps};

use crate::{
    ext::WithModel,
    model::{AppModel, DrumMachinePlaybackState},
    update,
    util::{resource_as_string, uuidize_builder_template},
    AppMessage, AppModelPtr, AsampoView,
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
    setup_drum_machine_view(model_ptr.clone(), view);

    view.sequences_add_sequence_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::AddSequenceClicked);
        }),
    );

    model_ptr.with_model(|model| {
        update_sequences_list(model_ptr.clone(), &model, view);
        update_drum_machine_view(&model);
        model
    });
}

pub fn update_sequences_list(model_ptr: AppModelPtr, model: &AppModel, view: &AsampoView) {
    view.sequences_list.remove_all();

    view.sequences_list_frame.set_label(Some(&format!(
        "Sequences ({})",
        model.sequences_map().len()
    )));

    for sequence in model.sequences_list().iter() {
        let uuid = sequence.uuid();

        let objects = gtk::Builder::from_string(&uuidize_builder_template(
            &resource_as_string("/sequences-list-row.ui").unwrap(),
            uuid,
        ));

        let row = objects
            .object::<gtk::ListBoxRow>(format!("{uuid}-row"))
            .unwrap();

        let name_label = objects
            .object::<gtk::Label>(format!("{uuid}-name-label"))
            .unwrap();

        name_label.set_text(model.sequence(uuid).unwrap().name());

        let clicked = GestureClick::new();

        clicked.connect_pressed(|e: &GestureClick, _, _, _| {
            e.widget().activate();
        });

        row.add_controller(clicked);

        let keyup = EventControllerKey::new();

        keyup.connect_key_released(clone!(@strong model_ptr, @strong view, @strong uuid =>
            move |_: &EventControllerKey, _, _, _| {
                update(model_ptr.clone(), &view, AppMessage::SequenceSelected(uuid));
            }
        ));

        row.add_controller(keyup);
        view.sequences_list.append(&row);

        if model
            .selected_sequence()
            .is_some_and(|sel_uuid| sel_uuid == uuid)
        {
            row.activate();
        }

        row.connect_activate(
            clone!(@strong model_ptr, @strong view, @strong uuid => move |_: &gtk::ListBoxRow| {
                update(model_ptr.clone(), &view, AppMessage::SequenceSelected(uuid));
            }),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrumMachineView {
    tempo_spinbutton: SpinButton,
    swing_spinbutton: SpinButton,
    play_button: Button,
    stop_button: Button,
    back_button: Button,
    save_seq_button: Button,
    save_set_button: Button,
    pad_buttons: [Button; 16],
    part_buttons: [Button; 4],
    step_buttons: [Button; 16],
}

fn setup_drum_machine_view(model_ptr: AppModelPtr, view: &AsampoView) {
    let objects = gtk::Builder::from_resource("/drum-machine.ui");

    macro_rules! connect {
        (spinner $name:expr, $x:ident => $message:expr) => {
            objects.object::<SpinButton>($name).unwrap().connect_value_changed(
                clone!(@strong model_ptr, @strong view => move |$x: &SpinButton| {
                    update(model_ptr.clone(), &view, $message);
                })
            );
        };

        (button $name:expr, $message:expr) => {
            objects.object::<Button>($name).unwrap().connect_clicked(
                clone!(@strong model_ptr, @strong view => move |_: &Button| {
                    update(model_ptr.clone(), &view, $message);
                })
            );
        };
    }

    macro_rules! obj {
        ($typ:ty, $name:expr) => {
            objects.object::<$typ>($name).unwrap()
        };
    }

    connect!(spinner "sequences-editor-tempo-entry",
        x => AppMessage::DrumMachineTempoChanged(x.value_as_int() as u16));
    connect!(spinner "sequences-editor-swing-entry",
        x => AppMessage::DrumMachineSwingChanged(x.value_as_int() as u32));

    connect!(button "sequences-editor-play-button", AppMessage::DrumMachinePlayClicked);
    connect!(button "sequences-editor-stop-button", AppMessage::DrumMachineStopClicked);
    connect!(button "sequences-editor-back-button", AppMessage::DrumMachineBackClicked);

    connect!(button "sequences-editor-save-seq-button",
        AppMessage::DrumMachineSaveSequenceClicked);
    connect!(button "sequences-editor-save-seq-as-button",
        AppMessage::DrumMachineSaveSequenceAsClicked);
    connect!(button "sequences-editor-save-set-button",
        AppMessage::DrumMachineSaveSampleSetClicked);
    connect!(button "sequences-editor-save-set-as-button",
        AppMessage::DrumMachineSaveSampleSetAsClicked);

    let mut pad_buttons: Vec<Button> = vec![];
    let mut part_buttons: Vec<Button> = vec![];
    let mut step_buttons: Vec<Button> = vec![];

    for (index, _label) in LABELS.into_iter().enumerate() {
        connect!(button format!("sequences-editor-pad-{}", index),
            AppMessage::DrumMachinePadClicked(index));

        pad_buttons.push(obj!(Button, format!("sequences-editor-pad-{}", index)));
    }

    for index in 0..4 {
        connect!(button format!("sequences-editor-part-{}", index),
            AppMessage::DrumMachinePartClicked(index));

        part_buttons.push(obj!(Button, format!("sequences-editor-part-{}", index)));
    }

    for index in 0..16 {
        connect!(button format!("sequences-editor-step-{}", index),
            AppMessage::DrumMachineStepClicked(index));

        step_buttons.push(obj!(Button, format!("sequences-editor-step-{}", index)));
    }

    let pad_buttons: [Button; 16] = pad_buttons.try_into().unwrap();
    let part_buttons: [Button; 4] = part_buttons.try_into().unwrap();
    let step_buttons: [Button; 16] = step_buttons.try_into().unwrap();

    let mut model = model_ptr.take().unwrap();
    model = model.set_drum_machine_view(Some(DrumMachineView {
        tempo_spinbutton: obj!(SpinButton, "sequences-editor-tempo-entry"),
        swing_spinbutton: obj!(SpinButton, "sequences-editor-swing-entry"),
        play_button: obj!(Button, "sequences-editor-play-button"),
        stop_button: obj!(Button, "sequences-editor-stop-button"),
        back_button: obj!(Button, "sequences-editor-back-button"),
        save_seq_button: obj!(Button, "sequences-editor-save-seq-button"),
        save_set_button: obj!(Button, "sequences-editor-save-set-button"),
        pad_buttons,
        part_buttons,
        step_buttons,
    }));
    model_ptr.replace(Some(model));

    let root = objects.object::<gtk::Box>("drum-machine-root").unwrap();

    view.sequences_editor_drum_machine_frame
        .set_child(Some(&root));
}

pub fn update_drum_machine_view(model: &AppModel) {
    let drum_machine_model = model.drum_machine_model();
    let drum_machine_view = model.drum_machine_view().unwrap();

    assert!(drum_machine_model.activated_pad() < 16);

    drum_machine_view
        .tempo_spinbutton
        .set_value(drum_machine_model.sequence().timespec().bpm.get() as f64);

    drum_machine_view
        .swing_spinbutton
        .set_value(drum_machine_model.sequence().timespec().swing.get() * 100.0);

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

    if drum_machine_model.loaded_sequence().is_some() {
        drum_machine_view.save_seq_button.set_label(
            format!(
                "Save to '{}'",
                drum_machine_model.loaded_sequence().unwrap().name()
            )
            .as_str(),
        );

        drum_machine_view.save_seq_button.set_sensitive(true);
    } else {
        drum_machine_view.save_seq_button.set_sensitive(false);
    }

    if drum_machine_model.is_waiting()
        || drum_machine_model.playback_state() == DrumMachinePlaybackState::Stopped
    {
        for (i, _) in LABELS.iter().enumerate() {
            drum_machine_view.step_buttons[i].remove_css_class("playing");
        }
    } else if let Some(event) = drum_machine_model.latest_event() {
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
