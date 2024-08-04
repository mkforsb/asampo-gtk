// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

#[derive(Debug, Clone)]
pub struct ViewFlags {
    view_sensitive: bool,
    sources_add_fs_fields_valid: bool,
    add_to_prev_enabled: bool,
    sets_load_in_drum_machine_enabled: bool,
    sets_export_enabled: bool,
    sets_export_fields_valid: bool,
}

impl Default for ViewFlags {
    fn default() -> Self {
        ViewFlags {
            view_sensitive: true,
            sources_add_fs_fields_valid: false,
            add_to_prev_enabled: false,
            sets_load_in_drum_machine_enabled: false,
            sets_export_enabled: false,
            sets_export_fields_valid: false,
        }
    }
}

impl ViewFlags {
    get_set!(are add_fs_source_fields_valid, sources_add_fs_fields_valid);
    get_set!(are export_fields_valid, sets_export_fields_valid);
    get_set!(is main_view_sensitive, view_sensitive);
    get_set!(is set_load_in_drum_machine_enabled, sets_load_in_drum_machine_enabled);
    get_set!(is set_export_enabled, sets_export_enabled);
    get_set!(is add_to_prev_set_enabled, add_to_prev_enabled);
}

/// Generates a pair of methods for a boolean field.
///
/// `get_set!(prefix foo, myfield)` -> `pub fn set_foo(state: bool)`
///                                    `pub fn prefix_foo()`
///
/// The `prefix` is intended to be e.g "is", such that the generated pair would be
/// e.g `set_elevator_stopped` and `is_elevator_stopped`.
macro_rules! get_set {
    ($pre:ident $topic:ident, $field:ident) => {
        paste::paste! {
            pub fn [<set_ $topic>](self, state: bool) -> Self { set_field!(self, $field, state) }
            pub fn [<$pre _ $topic>](&self) -> bool { self.$field }
        }
    };
}

use get_set;

/// Expands to struct update syntax that updates a single field.
///
/// `set_field!(self, foo, bar)` -> `Self { foo: bar, ..self }`
macro_rules! set_field {
    ($self:ident, $key:ident, $val:expr) => {
        Self {
            $key: $val,
            ..$self
        }
    };
}

use set_field;
