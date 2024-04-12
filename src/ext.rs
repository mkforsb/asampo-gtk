// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{cell::Cell, collections::HashMap};

use libasampo::sources::Source;
use uuid::Uuid;

pub trait WithModel<T, F>
where
    F: Fn(T) -> T,
{
    fn with_model(&self, f: F);
}

impl<T, F> WithModel<T, F> for Cell<Option<T>>
where
    F: Fn(T) -> T,
{
    fn with_model(&self, f: F) {
        let inner = self.take().unwrap();
        self.set(Some(f(inner)));
    }
}

pub trait ClonedUpdateWith<T, F>
where
    Self: Sized,
    F: Fn(Self) -> T,
{
    fn cloned_update_with(&self, f: F) -> T;
}

impl<T, F> ClonedUpdateWith<T, F> for HashMap<Uuid, Source>
where
    Self: Clone,
    F: Fn(Self) -> T,
{
    fn cloned_update_with(&self, f: F) -> T {
        f(self.clone())
    }
}
