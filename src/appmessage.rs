// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use gtk::gdk::ModifierType;
use libasampo::{samples::Sample, samplesets::DrumkitLabel, sequences::DrumkitSequenceEvent};
use uuid::Uuid;

use crate::view::dialogs::{self, InputDialogContext, SelectFolderDialogContext};

#[derive(Debug)]
pub enum AppMessage {
    TimerTick,
    SettingsOutputSampleRateChanged(String),
    SettingsBufferSizeChanged(u16),
    SettingsSampleRateConversionQualityChanged(String),
    SettingsSamplePlaybackBehaviorChanged(String),
    AddFilesystemSourceNameChanged(String),
    AddFilesystemSourcePathChanged(String),
    AddFilesystemSourcePathBrowseClicked,
    AddFilesystemSourcePathBrowseSubmitted(String),
    AddFilesystemSourcePathBrowseError(gtk::glib::Error),
    AddFilesystemSourceExtensionsChanged(String),
    AddFilesystemSourceClicked,
    SampleListSampleSelected(u32),
    SampleSetSampleSelected(Sample),
    SamplesFilterChanged(String),
    SampleSidebarAddToSetClicked,
    SampleSidebarAddToMostRecentlyUsedSetClicked,
    SourceEnabled(Uuid),
    SourceDisabled(Uuid),
    SourceDeleteClicked(Uuid),
    SourceLoadingMessage(Uuid, Vec<Result<Sample, libasampo::errors::Error>>),
    SourceLoadingDisconnected(Uuid),
    LoadFromSavefile(String),
    SaveToSavefile(String),
    DialogError(gtk::glib::Error),
    AddSampleSetClicked,
    InputDialogOpened(InputDialogContext),
    InputDialogSubmitted(InputDialogContext, String),
    InputDialogCanceled(InputDialogContext),
    SelectFolderDialogOpened(SelectFolderDialogContext),
    SampleSetSelected(Uuid),
    SampleSetSampleLabelChanged(Sample, Option<DrumkitLabel>),
    SampleSetDetailsLoadInDrumMachineClicked,
    SampleSetDetailsExportClicked,
    ExportDialogOpened(dialogs::ExportDialogView),
    ExportDialogClosed,
    ExportTargetDirectoryChanged(String),
    ExportTargetDirectoryBrowseClicked,
    ExportTargetDirectoryBrowseSubmitted(String),
    ExportTargetDirectoryBrowseError(gtk::glib::Error),
    PerformExportClicked,
    PlainCopyExportSelected,
    ConversionExportSelected,
    ExportJobMessage(libasampo::samplesets::export::ExportJobMessage),
    ExportJobDisconnected,
    StopAllSoundButtonClicked,
    DrumMachineTempoChanged(u16),
    DrumMachineSwingChanged(u32),
    DrumMachinePlayClicked,
    DrumMachineStopClicked,
    DrumMachineBackClicked,
    DrumMachineSaveSequenceClicked,
    DrumMachineSaveSequenceAsClicked,
    DrumMachineClearSequenceClicked,
    DrumMachineSaveSampleSetClicked,
    DrumMachineSaveSampleSetAsClicked,
    DrumMachineClearSampleSetClicked,
    DrumMachinePadClicked(usize),
    DrumMachinePartClicked(usize, ModifierType),
    DrumMachineStepClicked(usize),
    DrumMachinePlaybackEvent(DrumkitSequenceEvent),
    AssignSampleToPadClicked(usize),
    SequenceSelected(Uuid),
    AddSequenceClicked,
    LoadSequenceConfirmSaveDialogOpened,
    LoadSequenceConfirmSaveChanges,
    LoadSequenceConfirmDiscardChanges,
    LoadSequenceCancelSave,
    LoadSequenceConfirmAbandonDialogOpened,
    LoadSequenceConfirmAbandon,
    LoadSequenceCancelAbandon,
    LoadSequenceConfirmDialogError(anyhow::Error),
    ClearSequenceConfirmDialogOpened,
    ClearSequenceConfirmDialogError(anyhow::Error),
    ClearSequenceConfirm,
    ClearSequenceCancel,
}
