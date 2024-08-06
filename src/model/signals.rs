// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::collections::HashSet;

use anyhow::anyhow;

use crate::model::AnyhowResult;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Signal {
    ShowSampleSetCreateDialog,
    ShowSampleSetDeleteDialog,
    ShowAddSampleToSetDialog,
    ShowSampleSetSaveAsDialog,
    ShowSampleSetSaveBeforeLoadDialog,
    ShowSampleSetConfirmAbandonDialog,
    ShowSampleSetConfirmClearDialog,
    ShowSampleSetSynchronizationDialog,
    ShowExportDialog,
    ShowExportBrowseDialog,
    ShowAddFilesystemSourceBrowseDialog,
    ShowSequenceCreateDialog,
    ShowSequenceDeleteDialog,
    ShowSequenceSaveAsDialog,
    ShowSequenceSaveBeforeLoadDialog,
    ShowSequenceConfirmAbandonDialog,
    ShowSequenceConfirmClearDialog,
    ShowSaveBeforeQuitConfirmDialog,
    ShowSaveBeforeQuitSaveDialog,
    QuitConfirmed,
}

#[derive(Debug, Clone, Default)]
pub struct SignalModel {
    signals: HashSet<Signal>,
}

impl SignalModel {
    pub fn new() -> SignalModel {
        Default::default()
    }

    pub fn signal(self, signal: Signal) -> SignalModel {
        let mut new_signals = self.signals.clone();
        new_signals.insert(signal);

        SignalModel {
            signals: new_signals,
        }
    }

    pub fn clear_signal(self, signal: Signal) -> AnyhowResult<SignalModel> {
        let mut new_signals = self.signals.clone();

        if new_signals.remove(&signal) {
            Ok(SignalModel {
                signals: new_signals,
            })
        } else {
            Err(anyhow!("Signal not active"))
        }
    }

    pub fn is_signalling(&self, signal: Signal) -> bool {
        self.signals.contains(&signal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signals() {
        let model = SignalModel::new();

        let model = model.signal(Signal::ShowExportDialog);
        let model = model.signal(Signal::ShowSampleSetSaveAsDialog);

        assert!(model.is_signalling(Signal::ShowExportDialog));
        assert!(model.is_signalling(Signal::ShowSampleSetSaveAsDialog));

        assert!(!model.is_signalling(Signal::ShowSequenceConfirmClearDialog));

        let model = model.clear_signal(Signal::ShowExportDialog).unwrap();

        assert!(!model.is_signalling(Signal::ShowExportDialog));
        assert!(model.is_signalling(Signal::ShowSampleSetSaveAsDialog));

        assert!(model.clear_signal(Signal::ShowExportDialog).is_err());
    }
}
