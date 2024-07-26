// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

#[derive(Debug, Clone)]
pub struct ViewFlags {
    view_sensitive: bool,
    sources_add_fs_fields_valid: bool,
    sources_add_fs_begin_browse: bool,
    add_to_set_show_dialog: bool,
    add_to_prev_enabled: bool,
    sets_add_set_show_dialog: bool,
    sets_load_in_drum_machine_enabled: bool,
    sets_export_enabled: bool,
    sets_export_show_dialog: bool,
    sets_export_begin_browse: bool,
    sets_export_fields_valid: bool,
    sequences_create_sequence_show_dialog: bool,
    sequences_sequence_save_as_show_dialog: bool,
    sequences_sampleset_save_as_show_dialog: bool,
    sequences_load_sequence_show_confirm_save_dialog: bool,
    sequences_load_sequence_show_confirm_abandon_dialog: bool,
    sequences_clear_sequence_show_confirm_dialog: bool,
    sequences_clear_sampleset_show_confirm_dialog: bool,
}

impl Default for ViewFlags {
    fn default() -> Self {
        ViewFlags {
            view_sensitive: true,
            sources_add_fs_fields_valid: false,
            sources_add_fs_begin_browse: false,
            add_to_set_show_dialog: false,
            add_to_prev_enabled: false,
            sets_add_set_show_dialog: false,
            sets_load_in_drum_machine_enabled: false,
            sets_export_enabled: false,
            sets_export_show_dialog: false,
            sets_export_begin_browse: false,
            sets_export_fields_valid: false,
            sequences_create_sequence_show_dialog: false,
            sequences_sequence_save_as_show_dialog: false,
            sequences_sampleset_save_as_show_dialog: false,
            sequences_load_sequence_show_confirm_save_dialog: false,
            sequences_load_sequence_show_confirm_abandon_dialog: false,
            sequences_clear_sequence_show_confirm_dialog: false,
            sequences_clear_sampleset_show_confirm_dialog: false,
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

    signal!(add_sample_to_set_show_dialog, add_to_set_show_dialog);
    signal!(add_set_show_dialog, sets_add_set_show_dialog);
    signal!(export_begin_browse, sets_export_begin_browse);
    signal!(export_show_dialog, sets_export_show_dialog);
    signal!(add_fs_source_begin_browse, sources_add_fs_begin_browse);
    signal!(
        create_sequence_show_dialog,
        sequences_create_sequence_show_dialog
    );
    signal!(
        sequence_save_as_show_dialog,
        sequences_sequence_save_as_show_dialog
    );
    signal!(
        sampleset_save_as_show_dialog,
        sequences_sampleset_save_as_show_dialog
    );
    signal!(
        sequence_load_show_confirm_save_dialog,
        sequences_load_sequence_show_confirm_save_dialog
    );
    signal!(
        sequence_load_show_confirm_abandon_dialog,
        sequences_load_sequence_show_confirm_abandon_dialog
    );
    signal!(
        sequence_clear_show_confirm_dialog,
        sequences_clear_sequence_show_confirm_dialog
    );
    signal!(
        sampleset_clear_show_confirm_dialog,
        sequences_clear_sampleset_show_confirm_dialog
    );
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

/// Generates three methods for a boolean field.
///
/// `signal!(foo, myfield)` -> `pub fn signal_foo()`
///                            `pub fn clear_signal_foo()`
///                            `pub fn is_signalling_foo()`
macro_rules! signal {
    ($sig:ident, $field:ident) => {
        paste::paste! {
            pub fn [<signal_ $sig>](self) -> Self { set_field!(self, $field, true) }
            pub fn [<clear_signal_ $sig>](self) -> Self { set_field!(self, $field, false) }
            pub fn [<is_signalling_ $sig>](&self) -> bool { self.$field }
        }
    };
}

use signal;

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
