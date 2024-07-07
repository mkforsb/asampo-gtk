// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

#[derive(Debug, Clone)]
pub struct ViewFlags {
    view_sensitive: bool,
    sources_add_fs_fields_valid: bool,
    sources_add_fs_begin_browse: bool,
    samples_sidebar_add_to_set_show_dialog: bool,
    samples_sidebar_add_to_prev_enabled: bool,
    sets_add_set_show_dialog: bool,
    sets_export_enabled: bool,
    sets_export_show_dialog: bool,
    sets_export_begin_browse: bool,
    sets_export_fields_valid: bool,
}

impl Default for ViewFlags {
    fn default() -> Self {
        ViewFlags {
            view_sensitive: true,
            sources_add_fs_fields_valid: false,
            sources_add_fs_begin_browse: false,
            samples_sidebar_add_to_set_show_dialog: false,
            samples_sidebar_add_to_prev_enabled: false,
            sets_add_set_show_dialog: false,
            sets_export_enabled: false,
            sets_export_show_dialog: false,
            sets_export_begin_browse: false,
            sets_export_fields_valid: false,
        }
    }
}

impl ViewFlags {
    pub fn set_are_add_fs_source_fields_valid(self, valid: bool) -> ViewFlags {
        ViewFlags {
            sources_add_fs_fields_valid: valid,
            ..self
        }
    }

    pub fn signal_add_fs_source_begin_browse(self) -> ViewFlags {
        ViewFlags {
            sources_add_fs_begin_browse: true,
            ..self
        }
    }

    pub fn clear_signal_add_fs_source_begin_browse(self) -> ViewFlags {
        ViewFlags {
            sources_add_fs_begin_browse: false,
            ..self
        }
    }

    pub fn signal_add_sample_to_set_show_dialog(self) -> ViewFlags {
        ViewFlags {
            samples_sidebar_add_to_set_show_dialog: true,
            ..self
        }
    }

    pub fn clear_signal_add_sample_to_set_show_dialog(self) -> ViewFlags {
        ViewFlags {
            samples_sidebar_add_to_set_show_dialog: false,
            ..self
        }
    }

    pub fn enable_set_export(self) -> ViewFlags {
        ViewFlags {
            sets_export_enabled: true,
            ..self
        }
    }

    pub fn disable_set_export(self) -> ViewFlags {
        ViewFlags {
            sets_export_enabled: false,
            ..self
        }
    }

    pub fn is_set_export_enabled(&self) -> bool {
        self.sets_export_enabled
    }

    pub fn signal_add_set_show_dialog(self) -> ViewFlags {
        ViewFlags {
            sets_add_set_show_dialog: true,
            ..self
        }
    }

    pub fn clear_signal_add_set_show_dialog(self) -> ViewFlags {
        ViewFlags {
            sets_add_set_show_dialog: false,
            ..self
        }
    }

    pub fn signal_export_begin_browse(self) -> ViewFlags {
        ViewFlags {
            sets_export_begin_browse: true,
            ..self
        }
    }

    pub fn clear_signal_export_begin_browse(self) -> ViewFlags {
        ViewFlags {
            sets_export_begin_browse: false,
            ..self
        }
    }

    pub fn signal_export_show_dialog(self) -> ViewFlags {
        ViewFlags {
            sets_export_show_dialog: true,
            ..self
        }
    }

    pub fn clear_signal_export_show_dialog(self) -> ViewFlags {
        ViewFlags {
            sets_export_show_dialog: false,
            ..self
        }
    }

    pub fn set_main_view_sensitive(self, sensitive: bool) -> ViewFlags {
        ViewFlags {
            view_sensitive: sensitive,
            ..self
        }
    }

    pub fn set_are_export_fields_valid(self, valid: bool) -> ViewFlags {
        ViewFlags {
            sets_export_fields_valid: valid,
            ..self
        }
    }

    pub fn is_main_view_sensitive(&self) -> bool {
        self.view_sensitive
    }

    pub fn are_export_fields_valid(&self) -> bool {
        self.sets_export_fields_valid
    }

    pub fn is_signalling_add_fs_source_begin_browse(&self) -> bool {
        self.sources_add_fs_begin_browse
    }

    pub fn is_signalling_add_sample_to_set_show_dialog(&self) -> bool {
        self.samples_sidebar_add_to_set_show_dialog
    }

    pub fn is_signalling_add_set_show_dialog(&self) -> bool {
        self.sets_add_set_show_dialog
    }

    pub fn is_signalling_export_show_dialog(&self) -> bool {
        self.sets_export_show_dialog
    }

    pub fn is_signalling_export_begin_browse(&self) -> bool {
        self.sets_export_begin_browse
    }

    pub fn are_add_fs_source_fields_valid(&self) -> bool {
        self.sources_add_fs_fields_valid
    }

    pub fn enable_add_to_prev_set(self) -> ViewFlags {
        ViewFlags {
            samples_sidebar_add_to_prev_enabled: true,
            ..self
        }
    }

    pub fn disable_add_to_prev_set(self) -> ViewFlags {
        ViewFlags {
            samples_sidebar_add_to_prev_enabled: false,
            ..self
        }
    }

    pub fn is_add_to_prev_set_enabled(&self) -> bool {
        self.samples_sidebar_add_to_prev_enabled
    }
}
