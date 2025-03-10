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
    F: FnOnce(&T) -> U,
{
    fn peek_model(&self, f: F) -> U;
}

impl<T, U, F> PeekModel<T, U, F> for Cell<Option<T>>
where
    F: FnOnce(&T) -> U,
{
    fn peek_model(&self, f: F) -> U {
        let inner = self.take().unwrap();
        let res = f(&inner);
        self.set(Some(inner));
        res
    }
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
    fn clone_and_insert(&self, item: T, position: usize) -> Self;
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

    fn clone_and_insert(&self, item: T, position: usize) -> Self {
        if position > self.len() {
            self.clone_and_push(item)
        } else {
            let mut result = self.clone();
            result.insert(position, item);
            result
        }
    }
}

pub trait OptionMapExt<T> {
    fn value_for(&self, key: &str) -> Option<&T>;
    fn key_for(&self, value: &T) -> Option<&str>;
    fn keys(&self) -> Vec<&'static str>;
    fn values(&self) -> Vec<T>;
}

impl<T> OptionMapExt<T> for [(&'static str, T)]
where
    T: std::cmp::PartialEq + Copy,
{
    fn value_for(&self, key: &str) -> Option<&T> {
        self.iter().find(|(k, _v)| *k == key).map(|(_k, v)| v)
    }

    fn key_for(&self, value: &T) -> Option<&str> {
        self.iter().find(|(_k, v)| v == value).map(|(k, _v)| *k)
    }

    fn keys(&self) -> Vec<&'static str> {
        self.iter().map(|(key, _)| *key).collect()
    }

    fn values(&self) -> Vec<T> {
        self.iter().map(|(_, val)| *val).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Model {
        value: i32,
    }

    const OPTIONS: [(&str, i32); 3] = [("a", 1), ("b", -2), ("c", 1)];

    #[test]
    fn test_with_model() {
        Cell::new(Some(Model { value: 42 })).with_model(|model| {
            assert_eq!(model.value, 42);
            model
        });
    }

    #[test]
    fn test_peek_model() {
        assert_eq!(
            Cell::new(Some(Model { value: 42 })).peek_model(|model| model.value),
            42
        );
    }

    #[test]
    fn test_options_map_ext() {
        assert_eq!(OPTIONS.value_for("a"), Some(1).as_ref());
        assert_eq!(OPTIONS.value_for("b"), Some(-2).as_ref());
        assert_eq!(OPTIONS.value_for("c"), Some(1).as_ref());

        assert_eq!(OPTIONS.key_for(&1), Some("a"));
        assert_eq!(OPTIONS.key_for(&-2), Some("b"));

        assert_eq!(OPTIONS.keys(), vec!["a", "b", "c"]);
        assert_eq!(OPTIONS.values(), vec![1, -2, 1]);
    }
}
