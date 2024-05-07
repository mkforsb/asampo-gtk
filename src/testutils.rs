// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

#[cfg(test)]
pub mod savefile_for_test {
    use std::cell::Cell;

    use crate::model::AppModel;

    thread_local! {
        #[allow(clippy::type_complexity)]
        pub static SAVE: Cell<Option<fn(&AppModel, &str) -> Result<(), anyhow::Error>>>
            = Cell::new(None);

        #[allow(clippy::type_complexity)]
        pub static LOAD: Cell<Option<fn(&str) -> Result<AppModel, anyhow::Error>>>
            = Cell::new(None);
    }

    pub struct Savefile {}

    impl Savefile {
        pub fn save(model: &AppModel, filename: &str) -> Result<(), anyhow::Error> {
            SAVE.get()
                .expect("A function pointer should be placed in SAVE")(model, filename)
        }
        pub fn load(filename: &str) -> Result<AppModel, anyhow::Error> {
            LOAD.get()
                .expect("A function pointer should be placed in LOAD")(filename)
        }
    }
}
