// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::mpsc::{self, Sender},
    thread::JoinHandle,
};

use anyhow::anyhow;
use gtk::{
    gio::{ApplicationFlags, ListStore},
    glib::{clone, ExitCode},
    prelude::*,
    Application,
};

use libasampo::{
    prelude::*,
    samples::Sample,
    sources::{file_system_source::FilesystemSource, Source, SourceTrait},
};
use samples::{setup_samples_page, SampleListEntry};
use sources::setup_sources_page;
use sources::update_sources_list;
use uuid::Uuid;
use view::AsampoView;

mod ext;
mod samples;
mod sources;
mod view;

use ext::*;

#[derive(Debug, Clone)]
struct AppFlags {
    sources_add_fs_fields_valid: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for AppFlags {
    fn default() -> Self {
        AppFlags {
            sources_add_fs_fields_valid: false,
        }
    }
}

#[derive(Default, Debug, Clone)]
struct AppValues {
    sources_add_fs_name_entry: String,
    sources_add_fs_path_entry: String,
    sources_add_fs_extensions_entry: String,
    samples_list_filter: String,
}

#[derive(Clone, Debug)]
struct AppModel {
    flags: AppFlags,
    values: AppValues,
    audiothread_tx: Sender<audiothread::Message>,
    _audiothread_handle: Rc<JoinHandle<()>>,
    sources: HashMap<Uuid, Source>,
    sources_order: Vec<Uuid>,
    samples: Rc<RefCell<Vec<Sample>>>,
    samples_listview_model: ListStore,
}

type AppModelPtr = Rc<Cell<Option<AppModel>>>;

impl AppModel {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        AppModel {
            flags: AppFlags::default(),
            values: AppValues::default(),
            audiothread_tx: tx,
            _audiothread_handle: Rc::new(audiothread::spawn(rx, Some(audiothread::Opts::default().with_bufsize_n_stereo_samples(1024)))),
            sources: HashMap::new(),
            sources_order: Vec::new(),
            samples: Rc::new(RefCell::new(Vec::new())),
            samples_listview_model: ListStore::new::<samples::SampleListEntry>(),
        }
    }

    fn add_source(self, source: Source) -> Self {
        let mut new_sources_order = self.sources_order.clone();
        new_sources_order.push(*source.uuid());

        let mut new_sources = self.sources.clone();
        new_sources.insert(*source.uuid(), source);

        AppModel {
            sources_order: new_sources_order,
            sources: new_sources,
            ..self
        }
    }

    fn enable_source(self, uuid: Uuid) -> Result<Self, anyhow::Error> {
        self.samples.borrow_mut().extend(
            self.sources
                .get(&uuid)
                .ok_or_else(|| anyhow!("Failed to enable source: uuid not found!"))?
                .list()?,
        );

        Ok(AppModel {
            sources: self.sources.cloned_update_with(
                |mut s: HashMap<Uuid, Source>| -> Result<HashMap<Uuid, Source>, anyhow::Error> {
                    s.get_mut(&uuid)
                        .ok_or_else(|| anyhow!("Failed to enable source: uuid not found!"))?
                        .enable();
                    Ok(s)
                },
            )?,
            ..self
        })
    }

    fn disable_source(self, uuid: Uuid) -> Result<Self, anyhow::Error> {
        self.samples.borrow_mut().retain(|s| s.source_uuid() != Some(&uuid));

        Ok(AppModel {
            sources: self.sources.cloned_update_with(
                |mut s: HashMap<Uuid, Source>| -> Result<HashMap<Uuid, Source>, anyhow::Error> {
                    s.get_mut(&uuid)
                        .ok_or_else(|| anyhow!("Failed to disable source: uuid not found!"))?
                        .disable();
                    Ok(s)
                },
            )?,
            ..self
        })
    }

    fn populate_samples_list(self) -> Self {
        let filter = &self.values.samples_list_filter;
        self.samples_listview_model.remove_all();

        if filter.is_empty() {
            let samples = self
                .samples
                .borrow()
                .iter()
                .map(|s| SampleListEntry::new(s.clone()))
                .collect::<Vec<_>>();

            self.samples_listview_model
                .extend_from_slice(samples.as_slice());
        } else {
            let fragments = filter.split(' ').map(|s| s.to_string()).collect::<Vec<_>>();

            let mut samples = self.samples.borrow().clone();
            samples.retain(|x| fragments.iter().all(|frag| x.uri().contains(frag)));

            self.samples_listview_model.extend_from_slice(
                samples
                    .iter()
                    .map(|s| SampleListEntry::new(s.clone()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
        }

        log::log!(
            log::Level::Debug,
            "showing {} samples",
            self.samples_listview_model.n_items()
        );

        self
    }
}

#[derive(Debug)]
enum AppMessage {
    AddFilesystemSourceNameChanged(String),
    AddFilesystemSourcePathChanged(String),
    AddFilesystemSourceExtensionsChanged(String),
    AddFilesystemSourceClicked,
    SampleClicked(u32),
    SamplesFilterChanged(String),
    SourceEnabled(Uuid),
    SourceDisabled(Uuid),
}

fn update(model_ptr: AppModelPtr, view: &AsampoView, message: AppMessage) {
    log::log!(log::Level::Debug, "{message:?}");

    let old_model = model_ptr.take().unwrap();

    match update_model(old_model.clone(), message) {
        Ok(new_model) => {
            model_ptr.set(Some(new_model.clone()));
            update_view(model_ptr, old_model, new_model, view);
        }

        Err(e) => {
            model_ptr.set(Some(old_model));
            log::log!(log::Level::Error, "{}", e.to_string());
        }
    }
}

fn update_model(model: AppModel, message: AppMessage) -> Result<AppModel, anyhow::Error> {
    fn check_sources_add_fs_valid(model: AppModel) -> AppModel {
        #[allow(clippy::needless_update)]
        AppModel {
            flags: AppFlags {
                sources_add_fs_fields_valid: !model.values.sources_add_fs_name_entry.is_empty()
                    && !model.values.sources_add_fs_path_entry.is_empty()
                    && !model.values.sources_add_fs_extensions_entry.is_empty(),
                ..model.flags
            },
            ..model
        }
    }

    match message {
        AppMessage::AddFilesystemSourceNameChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                values: AppValues {
                    sources_add_fs_name_entry: text,
                    ..model.values
                },
                ..model
            }))
        }

        AppMessage::AddFilesystemSourcePathChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                values: AppValues {
                    sources_add_fs_path_entry: text,
                    ..model.values
                },
                ..model
            }))
        }

        AppMessage::AddFilesystemSourceExtensionsChanged(text) => {
            Ok(check_sources_add_fs_valid(AppModel {
                values: AppValues {
                    sources_add_fs_extensions_entry: text,
                    ..model.values
                },
                ..model
            }))
        }

        // TODO: more validation, e.g is the path readable
        AppMessage::AddFilesystemSourceClicked => {
            let new_source = Source::FilesystemSource(FilesystemSource::new_named(
                model.values.sources_add_fs_name_entry.clone(),
                model.values.sources_add_fs_path_entry.clone(),
                model
                    .values
                    .sources_add_fs_extensions_entry
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            ));

            let uuid = *new_source.uuid();
            let model = model.add_source(new_source).enable_source(uuid).unwrap();

            Ok(AppModel {
                #[allow(clippy::needless_update)]
                flags: AppFlags {
                    sources_add_fs_fields_valid: false,
                    ..model.flags
                },

                values: AppValues {
                    sources_add_fs_name_entry: String::from(""),
                    sources_add_fs_path_entry: String::from(""),
                    sources_add_fs_extensions_entry: String::from(""),
                    ..model.values
                },

                ..model
            }
            .populate_samples_list())
        }

        AppMessage::SampleClicked(index) => {
            let item = model.samples_listview_model.item(index);

            match item
                .and_dynamic_cast_ref::<SampleListEntry>()
                .map(|x| &x.value)
            {
                Some(sample) => {
                    model
                        .audiothread_tx
                        .send(audiothread::Message::PlaySymphoniaSource(
                            audiothread::SymphoniaSource::from_file(sample.borrow().uri())?,
                        ))?;

                    Ok(model)
                }
                None => Err(anyhow!("Could not obtain clicked sample (this is a bug)")),
            }
        }

        AppMessage::SamplesFilterChanged(text) => Ok(AppModel {
            values: AppValues {
                samples_list_filter: text,
                ..model.values
            },
            ..model
        }.populate_samples_list()),

        AppMessage::SourceEnabled(uuid) => Ok(model.enable_source(uuid)?.populate_samples_list()),

        AppMessage::SourceDisabled(uuid) => Ok(model.disable_source(uuid)?.populate_samples_list()),
    }
}

fn update_view(model_ptr: AppModelPtr, old: AppModel, new: AppModel, view: &AsampoView) {
    macro_rules! maybe_update_entry_text {
        ($old:ident, $new:ident, $view:ident, $entry:ident) => {
            if $old.values.$entry != $new.values.$entry && $view.$entry.text() != $new.values.$entry
            {
                $view.$entry.set_text(&$new.values.$entry);
            }
        };
    }

    maybe_update_entry_text!(old, new, view, sources_add_fs_name_entry);
    maybe_update_entry_text!(old, new, view, sources_add_fs_path_entry);
    maybe_update_entry_text!(old, new, view, sources_add_fs_extensions_entry);

    if old.flags.sources_add_fs_fields_valid != new.flags.sources_add_fs_fields_valid {
        view.sources_add_fs_add_button
            .set_sensitive(new.flags.sources_add_fs_fields_valid);
    }

    if old.sources != new.sources {
        update_sources_list(model_ptr, new, view);
    }
}

fn main() -> ExitCode {
    env_logger::init();

    gtk::gio::resources_register_include!("resources.gresource")
        .expect("Should be able to register compiled GTK resources.");

    let app = Application::builder()
        .application_id("se.neode.Asampo")
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_command_line(clone!(@strong app =>  move |_, _| {
        app.activate();
        0
    }));

    app.connect_activate(|app| {
        let view = AsampoView::new(app);
        view.present();

        let model = Rc::new(Cell::new(Some(AppModel::new())));

        setup_sources_page(model.clone(), &view);
        setup_samples_page(model.clone(), &view);
    });

    app.run()
}
