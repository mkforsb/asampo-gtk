// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use anyhow::anyhow;
use gtk::{glib::object::IsA, prelude::*};
use uuid::Uuid;

use crate::ext::OptionMapExt;

const GIBIBYTE: u64 = 1024 * 1024 * 1024;
const MEBIBYTE: u64 = 1024 * 1024;
const KIBIBYTE: u64 = 1024;

pub fn readable_size(n_bytes: Option<u64>) -> String {
    if let Some(n) = n_bytes {
        if n >= GIBIBYTE {
            format!("{:.1} GiB", (n as f64) / (GIBIBYTE as f64))
        } else if n > MEBIBYTE {
            format!("{:.1} MiB", (n as f64) / (MEBIBYTE as f64))
        } else if n > KIBIBYTE {
            format!("{:.1} KiB", (n as f64) / (KIBIBYTE as f64))
        } else {
            format!("{} byte", n)
        }
    } else {
        "Unknown".to_string()
    }
}

const SECOND: u64 = 1000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;

pub fn readable_length(millis: Option<u64>) -> String {
    if let Some(mut ms) = millis {
        if ms >= MINUTE {
            let mut fragments = Vec::<String>::new();

            if ms >= 2 * HOUR {
                fragments.push(format!("{} hours", ms / HOUR));
                ms %= HOUR;
            } else if ms >= HOUR {
                fragments.push(format!("{} hour", ms / HOUR));
                ms %= HOUR;
            }

            if ms >= 2 * MINUTE {
                fragments.push(format!("{} minutes", ms / MINUTE));
                ms %= MINUTE;
            } else if ms >= MINUTE {
                fragments.push(format!("{} minute", ms / MINUTE));
                ms %= MINUTE;
            }

            if ms >= 2 * SECOND {
                fragments.push(format!("{} seconds", ms / SECOND));
            } else if ms >= SECOND {
                fragments.push(format!("{} second", ms / SECOND));
            }

            fragments.join(", ")
        } else {
            format!("{:.1} seconds", (ms as f64) / (SECOND as f64))
        }
    } else {
        "Unknown".to_string()
    }
}

pub fn gtk_find_child_by_builder_id<T>(root: &impl IsA<gtk::Widget>, id: &str) -> Option<T>
where
    T: gtk::prelude::ObjectType,
{
    let buildable_id = root
        .dynamic_cast_ref::<gtk::Buildable>()
        .unwrap()
        .buildable_id();

    if let Some(id_str) = buildable_id {
        if id_str == id {
            return root.clone().dynamic_cast::<T>().ok();
        }
    }

    let mut child = root.first_child();

    if child.is_some() {
        loop {
            if let Some(widget) = gtk_find_child_by_builder_id(child.as_ref().unwrap(), id) {
                return Some(widget);
            }

            child = child.as_ref().unwrap().next_sibling();

            if child.is_none() {
                break;
            }
        }
    }

    None
}

pub fn strs_dropdown_get_selected(e: &gtk::DropDown) -> String {
    e.model()
        .expect("Dropdown should have a model")
        .item(e.selected())
        .expect("Selected item should be obtainable from model")
        .dynamic_cast_ref::<gtk::StringObject>()
        .expect("ListModel should contain StringObject items")
        .string()
        .to_string()
}

pub fn set_dropdown_choice<T: PartialEq + Copy>(
    dropdown: &gtk::DropDown,
    options: &[(&'static str, T)],
    choice: &T,
) {
    let key = (*options)
        .key_for(choice)
        .expect("Active choice should have an associated key");

    if let Some(position) = dropdown
        .model()
        .expect("Dropdown should have a model")
        .iter()
        .position(|x: Result<gtk::glib::Object, _>| {
            x.expect("ListModel should not be mutated while iterating")
                .dynamic_cast_ref::<gtk::StringObject>()
                .expect("ListModel should contain StringObject items")
                .string()
                == key
        })
    {
        dropdown.set_selected(position.try_into().unwrap());
    }
}

pub fn resource_as_string(path: &str) -> Result<String, anyhow::Error> {
    let mut buffer = [0u8; 1048576];

    gtk::gio::resources_open_stream(path, gtk::gio::ResourceLookupFlags::NONE)?
        .read_all(&mut buffer, None::<&gtk::gio::Cancellable>)?;

    let end = buffer
        .iter()
        .position(|x| *x == 0)
        .ok_or(anyhow!("Resource too large"))?;

    Ok(String::from_utf8(buffer[0..end].to_vec())?)
}

pub fn uuidize_builder_template(xml: &str, uuid: Uuid) -> String {
    xml.replace("{uuid}", &format!("{uuid}"))
}

pub fn idize_builder_template(xml: &str, id: usize) -> String {
    xml.replace("{id}", &format!("{id}"))
}
