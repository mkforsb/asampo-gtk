// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

use std::{cell::Cell, collections::HashMap};

use anyhow::anyhow;

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

    ($model:expr, $field:ident.$subfield:ident) => {
        $model.peek_model(|model| {
            let res = model.$field.$subfield;
            (model, res)
        })
    };
}

pub trait ClonedHashMapExt<K, V>
where
    Self: Clone,
    K: Eq + std::hash::Hash,
{
    fn cloned_update_with<T, F>(&self, f: F) -> Result<T, anyhow::Error>
    where
        F: FnOnce(Self) -> Result<T, anyhow::Error>;

    fn clone_and_remove(&self, key: &K) -> Result<Self, anyhow::Error>;
    fn clone_and_insert(&self, key: K, value: V) -> Self;
}

impl<K, V> ClonedHashMapExt<K, V> for HashMap<K, V>
where
    Self: Clone,
    K: Eq + std::hash::Hash,
{
    fn cloned_update_with<T, F>(&self, f: F) -> Result<T, anyhow::Error>
    where
        F: FnOnce(Self) -> Result<T, anyhow::Error>,
    {
        f(self.clone())
    }

    fn clone_and_remove(&self, key: &K) -> Result<Self, anyhow::Error> {
        let mut result = self.clone();
        result.remove(key).ok_or(anyhow!("Key not present"))?;
        Ok(result)
    }

    fn clone_and_insert(&self, key: K, value: V) -> Self {
        let mut result = self.clone();
        result.insert(key, value);
        result
    }
}

pub trait ClonedVecExt<T> {
    fn clone_and_remove(&self, item: &T) -> Result<Self, anyhow::Error>
    where
        Self: Sized;

    fn clone_and_push(&self, item: T) -> Self;
}

impl<T> ClonedVecExt<T> for Vec<T>
where
    T: Clone + Eq,
{
    fn clone_and_remove(&self, item: &T) -> Result<Self, anyhow::Error> {
        let mut result = self.clone();

        let index = result
            .iter()
            .position(|val| val == item)
            .ok_or(anyhow!("Item not found"))?;

        result.remove(index);
        Ok(result)
    }

    fn clone_and_push(&self, item: T) -> Self {
        let mut result = self.clone();
        result.push(item);
        result
    }
}
