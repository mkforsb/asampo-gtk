// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{cell::Cell, collections::HashMap};

use libasampo::sources::Source;
use uuid::Uuid;

pub trait WithModel<T, F>
where
    F: FnOnce(T) -> T,
{
    fn with_model(&self, f: F);
}

impl<T, F> WithModel<T, F> for Cell<Option<T>>
where
    F: FnOnce(T) -> T,
{
    fn with_model(&self, f: F) {
        let inner = self.take().unwrap();
        self.set(Some(f(inner)));
    }
}

pub trait PeekModel<T, U, F>
where
    F: FnOnce(T) -> (T, U),
{
    fn peek_model(&self, f: F) -> U;
}

impl<T, U, F> PeekModel<T, U, F> for Cell<Option<T>>
where
    F: FnOnce(T) -> (T, U),
{
    fn peek_model(&self, f: F) -> U {
        let inner = self.take().unwrap();
        let (me, res) = f(inner);
        self.set(Some(me));
        res
    }
}

macro_rules! peek_model {
    ($model:expr, $field:ident) => {
        $model.peek_model(|model| {
            let res = model.$field;
            (model, res)
        })
    };
}

pub trait ClonedUpdateWith<T, F>
where
    Self: Sized,
    F: FnOnce(Self) -> T,
{
    fn cloned_update_with(&self, f: F) -> T;
}

impl<T, F> ClonedUpdateWith<T, F> for HashMap<Uuid, Source>
where
    Self: Clone,
    F: FnOnce(Self) -> T,
{
    fn cloned_update_with(&self, f: F) -> T {
        f(self.clone())
    }
}
