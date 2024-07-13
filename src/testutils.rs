// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

#[cfg(test)]
pub mod savefile_for_test {
    use std::cell::Cell;

    use libasampo::{samplesets::SampleSet, sequences::DrumkitSequence, sources::Source};

    use crate::model::AppModel;

    type AnyhowResult<T> = Result<T, anyhow::Error>;

    thread_local! {
        #[allow(clippy::type_complexity)]
        pub static SAVE: Cell<Option<fn(&AppModel, &str) -> AnyhowResult<()>>>
            = Cell::new(None);

        #[allow(clippy::type_complexity)]
        pub static LOAD: Cell<Option<fn(&str) -> AnyhowResult<Savefile>>>
            = Cell::new(None);
    }

    pub struct Savefile {
        pub sources_domained: Vec<Source>,
        pub sets_domained: Vec<SampleSet>,
        pub sequences_domained: Vec<DrumkitSequence>,
    }

    impl Savefile {
        pub fn save(model: &AppModel, filename: &str) -> AnyhowResult<()> {
            SAVE.get()
                .expect("A function pointer should be placed in SAVE")(model, filename)
        }

        pub fn load(filename: &str) -> AnyhowResult<Savefile> {
            LOAD.get()
                .expect("A function pointer should be placed in LOAD")(filename)
        }

        pub fn sources_domained(&self) -> AnyhowResult<Vec<Source>> {
            Ok(self.sources_domained.clone())
        }

        pub fn sets_domained(&self) -> AnyhowResult<Vec<SampleSet>> {
            Ok(self.sets_domained.clone())
        }

        pub fn sequences_domained(&self) -> AnyhowResult<Vec<DrumkitSequence>> {
            Ok(self.sequences_domained.clone())
        }
    }
}
