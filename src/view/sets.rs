// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::cell::RefCell;

use gtk::{
    glib::{self, clone, Object, Properties},
    prelude::*,
    subclass::prelude::*,
    EventControllerKey, GestureClick, Orientation,
};
use libasampo::{
    samples::{Sample, SampleOps},
    samplesets::{DrumkitLabel, SampleSetOps},
};

use crate::{
    appmessage::AppMessage,
    ext::{OptionMapExt, PeekModel, WithModel},
    labels::DRUM_LABELS,
    model::{AppModel, AppModelPtr, Signal},
    update,
    util::{resource_as_string, uuidize_builder_template},
    view::AsampoView,
};

#[derive(Debug, Default, Properties)]
#[properties(wrapper_type = MemberListEntry)]
pub struct MemberListEntryState {
    pub sample: RefCell<Sample>,

    #[property(get, set)]
    pub label_button_text: RefCell<String>,
}

#[glib::derived_properties]
impl ObjectImpl for MemberListEntryState {}

#[glib::object_subclass]
impl ObjectSubclass for MemberListEntryState {
    const NAME: &'static str = "MemberListEntry";
    type Type = MemberListEntry;
}

glib::wrapper! {
    pub struct MemberListEntry(ObjectSubclass<MemberListEntryState>);
}

impl MemberListEntry {
    pub fn new(sample: Sample, label: Option<DrumkitLabel>) -> Self {
        let obj = Object::builder()
            .property(
                "label_button_text",
                format!(
                    "Label: {}",
                    label
                        .and_then(|lb| DRUM_LABELS.key_for(&lb))
                        .unwrap_or("(None)")
                ),
            )
            .build();

        let x = MemberListEntryState::from_obj(&obj);
        x.sample.replace(sample);
        obj
    }
}

impl std::ops::Deref for MemberListEntry {
    type Target = MemberListEntryState;

    fn deref(&self) -> &Self::Target {
        MemberListEntryState::from_obj(self)
    }
}

pub fn setup_sets_page(model_ptr: AppModelPtr, view: &AsampoView) {
    fn select_member(
        model_ptr: AppModelPtr,
        view: &AsampoView,
        selectmodel: &gtk::SingleSelection,
    ) {
        let _ = selectmodel
            .item(selectmodel.selected())
            .and_then(|item| {
                item.downcast_ref::<MemberListEntry>()
                    .map(|entry| entry.sample.borrow().clone())
            })
            .map(|sample| {
                update(
                    model_ptr.clone(),
                    view,
                    AppMessage::SampleSetMemberSelected(sample),
                )
            });
    }

    view.sets_add_set_button
        .connect_clicked(clone!(@strong model_ptr, @strong view => move |_| {
            update(model_ptr.clone(), &view, AppMessage::AddSampleSetClicked);
        }));

    view.sets_details_load_drum_machine_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::SampleSetDetailsLoadInDrumMachineClicked);
        }),
    );

    view.sets_details_export_button.connect_clicked(
        clone!(@strong model_ptr, @strong view => move |_: &gtk::Button| {
            update(model_ptr.clone(), &view, AppMessage::SampleSetDetailsExportClicked);
        }),
    );

    let selectmodel = gtk::SingleSelection::new(Some(
        model_ptr.peek_model(|model| model.sets_members_listmodel().clone()),
    ));

    selectmodel.set_autoselect(false);

    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(clone!(
        @strong model_ptr,
        @strong view,
        @weak selectmodel => move |_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().expect("ListItem");
            let rowbox = gtk::Box::new(Orientation::Horizontal, 0);
            rowbox.set_homogeneous(false);

            unsafe {
                rowbox.set_data::<u32>("entry-index", list_item.position());
                rowbox.set_data::<bool>("bound", false);
            }

            let label = gtk::Label::new(None);
            label.set_xalign(0.0);
            label.set_hexpand(true);
            label.set_halign(gtk::Align::Fill);

            let clicked = GestureClick::new();

            clicked.connect_released(
                clone!(@strong model_ptr, @strong view, @weak selectmodel => move |_, _, _, _| {
                    select_member(model_ptr.clone(), &view, &selectmodel);
                }),
            );

            clicked.connect_unpaired_release(
                clone!(@strong model_ptr, @strong view, @weak selectmodel => move |_, _, _, _, _| {
                    select_member(model_ptr.clone(), &view, &selectmodel);
                }),
            );

            label.add_controller(clicked);

            rowbox.append(&label);

            list_item.set_child(Some(&rowbox));
        }
    ));

    factory.connect_bind(clone!(@weak model_ptr, @weak view => move |_, list_item| {
        let list_item = list_item.downcast_ref::<gtk::ListItem>().expect("ListItem");

        let rowbox = list_item
            .child()
            .and_downcast::<gtk::Box>()
            .expect("Box");

        let entry_index = list_item.position();

        let prev_entry_index = unsafe {
            *rowbox.data::<u32>("entry-index")
                .expect("Rowbox should have been assigned data `entry-index` by factory setup")
                .as_ptr()
        };

        let prev_bound = unsafe {
            *rowbox.data::<bool>("bound")
                .expect("Rowbox should have been assigned data `bound` by factory setup")
                .as_ptr()
        };

        if !prev_bound {
            unsafe { rowbox.set_data::<bool>("bound", true); }
        }

        if entry_index != prev_entry_index {
            if prev_bound {
                rowbox.remove(
                    &rowbox.last_child()
                        .expect("Previously-bound rowbox should have an opsbox to remove")
                );
            }

            let entry = list_item
                .item()
                .and_downcast::<MemberListEntry>()
                .expect("Entry");

            let sample = entry.sample.borrow().clone();

            let set_uuid = model_ptr.peek_model(|model|
                model.selected_set().expect("A set should be selected")
            );

            let opsbox = gtk::Box::new(Orientation::Horizontal, 0);
            opsbox.add_css_class("opsbox");

            let label_select_button = gtk::Button::new();
            label_select_button.add_css_class("label-select-button");
            label_select_button.set_hexpand(false);
            label_select_button.set_halign(gtk::Align::End);

            let label_select_button_inner_box = gtk::Box::new(Orientation::Horizontal, 0);
            let label_select_button_label = gtk::Label::new(None);

            entry.bind_property("label_button_text", &label_select_button_label, "label")
                .sync_create()
                .build();

            let label_select_button_icon = gtk::Image::new();
            label_select_button_icon.set_hexpand(true);
            label_select_button_icon.set_halign(gtk::Align::End);
            label_select_button_icon.set_icon_name(Some("pan-down-symbolic"));

            label_select_button_inner_box.append(&label_select_button_label);
            label_select_button_inner_box.append(&label_select_button_icon);

            // dummy element
            label_select_button_inner_box.append(&gtk::Popover::new());

            label_select_button.set_child(Some(&label_select_button_inner_box));

            label_select_button.connect_clicked(clone!(
                @weak model_ptr,
                @weak view,
                @strong sample,
                @strong set_uuid => move |sel_but: &gtk::Button| {
                    let popover = gtk::Popover::new();
                    popover.add_css_class("label-select-popover");

                    let popover_box = gtk::Box::new(Orientation::Vertical, 0);

                    let current_label = model_ptr.peek_model(|model|
                        model.set(set_uuid).expect("The selected set should exist")
                            .get_label::<DrumkitLabel>(&sample)
                            .expect("Label query should succeed for sample in selected set")
                            .and_then(|label| DRUM_LABELS.key_for(&label))
                            .unwrap_or("(None)")
                    );

                    for s in vec!["(None)"].into_iter().chain(DRUM_LABELS.keys()) {
                        let label = DRUM_LABELS.value_for(s).copied();
                        let button = gtk::Button::new();
                        button.set_hexpand(true);
                        button.set_halign(gtk::Align::Fill);

                        button.connect_clicked(clone!(
                            @weak model_ptr,
                            @weak view,
                            @weak popover,
                            @strong sample => move |_| {
                                popover.popdown();

                                model_ptr.with_model(|model|
                                    model.signal(Signal::SkipNextSampleSetMemberListUpdate)
                                );

                                update(
                                    model_ptr.clone(),
                                    &view,
                                    AppMessage::SampleSetMemberLabelChanged(
                                        sample.clone(),
                                        label
                                    )
                                );
                            }
                        ));

                        let button_inner_box = gtk::Box::new(Orientation::Horizontal, 0);
                        let button_label = gtk::Label::new(Some(s));

                        let button_icon = gtk::Image::new();
                        button_icon.set_icon_name(Some("object-select-symbolic"));

                        if s == current_label {
                            button.add_css_class("selected");
                        } else {
                            button_icon.set_visible(false);
                        }

                        button_inner_box.append(&button_label);
                        button_inner_box.append(&button_icon);

                        button.set_child(Some(&button_inner_box));
                        popover_box.append(&button);
                    }

                    let popover_wtf = gtk::Frame::new(None);
                    popover_wtf.set_child(Some(&popover_box));

                    popover.set_child(Some(&popover_wtf));

                    let label_select_button_inner_box = sel_but.first_child()
                        .and_downcast_ref::<gtk::Box>()
                        .expect("The label-select button should have a Box as first-child")
                        .clone();

                    label_select_button_inner_box.remove(
                        &label_select_button_inner_box
                            .last_child()
                            .expect("Either a dummy element or a previous popover instance \
                                should be present")
                    );

                    label_select_button_inner_box.append(&popover);
                    popover.popup();
                }
            ));

            let find_button = gtk::Button::new();
            find_button.set_icon_name("edit-find-symbolic");

            let delete_button = gtk::Button::new();
            delete_button.set_icon_name("user-trash-symbolic");

            delete_button.connect_clicked(clone!(
                @weak model_ptr,
                @weak view,
                @strong sample,
                @strong set_uuid => move |_| {
                    model_ptr.with_model(|model| {
                        model.signal(Signal::SkipNextSampleSetMemberListUpdate)
                    });

                    let listmodel = model_ptr
                        .peek_model(|model| model.sets_members_listmodel().clone());

                    listmodel.remove(entry_index);

                    view.sets_details_sample_list_frame
                        .set_label(Some(&format!("Samples ({})", listmodel.n_items())));

                    update(
                        model_ptr.clone(),
                        &view,
                        AppMessage::DeleteSampleFromSetClicked(sample.clone(), set_uuid)
                    );
                }
            ));

            opsbox.append(&label_select_button);
            opsbox.append(&find_button);
            opsbox.append(&delete_button);

            rowbox.append(&opsbox);

            let label = rowbox
                .first_child()
                .and_downcast::<gtk::Label>()
                .expect("Label");

            label.set_label(entry.sample.borrow().name());

            unsafe {
                rowbox.set_data::<u32>("entry-index", list_item.position());
            }
        }
    }));

    view.sets_details_sample_list.set_model(Some(&selectmodel));
    view.sets_details_sample_list.set_factory(Some(&factory));

    let keyed = EventControllerKey::new();

    keyed.connect_key_released(clone!(
        @strong model_ptr,
        @strong view,
        @weak selectmodel => move |_, key: gtk::gdk::Key, _, _| {
            if key == gtk::gdk::Key::Return {
                return;
            }

            select_member(model_ptr.clone(), &view, &selectmodel)
        }
    ));

    view.sets_details_sample_list.add_controller(keyed);
}

pub fn update_samplesets_list(model_ptr: AppModelPtr, model: AppModel, view: &AsampoView) {
    view.sets_list.remove_all();

    view.sets_list_frame
        .set_label(Some(&format!("Sets ({})", model.sets_map().len())));

    for set in model.sets_list().iter() {
        let uuid = set.uuid();

        let objects = gtk::Builder::from_string(&uuidize_builder_template(
            &resource_as_string("/sets-list-row.ui").unwrap(),
            uuid,
        ));

        let row = objects
            .object::<gtk::ListBoxRow>(format!("{uuid}-row"))
            .unwrap();

        let name_label = objects
            .object::<gtk::Label>(format!("{uuid}-name-label"))
            .unwrap();

        name_label.set_text(model.set(uuid).unwrap().name());

        let clicked = GestureClick::new();

        clicked.connect_pressed(clone!(@weak row => move |_, _, _, _| {
            row.activate();
        }));

        name_label.add_controller(clicked);

        let delete_button = objects
            .object::<gtk::Button>(format!("{uuid}-delete-button"))
            .unwrap();

        delete_button.connect_clicked(
            clone!(@strong model_ptr, @strong view, @strong uuid => move |_| {
                update(model_ptr.clone(), &view, AppMessage::SampleSetDeleteClicked(uuid))
            }),
        );

        let keyup = EventControllerKey::new();

        keyup.connect_key_released(clone!(@strong model_ptr, @strong view, @strong uuid =>
            move |_: &EventControllerKey, _, _, _| {
                update(model_ptr.clone(), &view, AppMessage::SampleSetSelected(uuid));
            }
        ));

        row.add_controller(keyup);

        view.sets_list.append(&row);

        if Some(uuid) == model.selected_set() {
            row.activate();
        }

        row.connect_activate(
            clone!(@strong model_ptr, @strong view, @strong uuid => move |_: &gtk::ListBoxRow| {
                update(model_ptr.clone(), &view, AppMessage::SampleSetSelected(uuid));
            }),
        );
    }
}

pub fn update_samplesets_detail(model_ptr: AppModelPtr, model: &AppModel, view: &AsampoView) {
    if model_ptr.peek_model(|model| model.is_signalling(Signal::SkipNextSampleSetMemberListUpdate))
    {
        model_ptr.with_model(|model| model.clear_signal(Signal::SkipNextSampleSetMemberListUpdate));
        return;
    }

    model.sets_members_listmodel().remove_all();

    match model.selected_set().and_then(|uuid| model.set(uuid).ok()) {
        Some(set) => {
            view.sets_details_name_label.set_text(set.name());

            view.sets_details_sample_list_frame
                .set_label(Some(&format!("Samples ({})", set.len())));

            model.sets_members_listmodel().extend_from_slice(
                set.list()
                    .iter()
                    .map(|sample| {
                        MemberListEntry::new(
                            (*sample).clone(),
                            set.get_label::<DrumkitLabel>(sample)
                                .expect("Label query should succeed for every sample in a set"),
                        )
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
        }

        None => {
            view.sets_details_name_label.set_text("");
        }
    }
}
