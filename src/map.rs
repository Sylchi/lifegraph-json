#[cfg(not(feature = "std"))]
use alloc::borrow::ToOwned;
#[cfg(not(feature = "std"))]
use alloc::string::ToString;

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::JsonValue;
use core::ops::{Deref, DerefMut};

#[derive(Clone, Debug, PartialEq)]
pub struct Map(pub(crate) Vec<(String, JsonValue)>);

impl Map {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub fn keys(&self) -> impl ExactSizeIterator<Item = &String> {
        self.0.iter().map(|(key, _)| key)
    }

    #[must_use]
    pub fn values(&self) -> impl ExactSizeIterator<Item = &JsonValue> {
        self.0.iter().map(|(_, value)| value)
    }

    pub fn values_mut(&mut self) -> impl ExactSizeIterator<Item = &mut JsonValue> {
        self.0.iter_mut().map(|(_, value)| value)
    }

    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &(String, JsonValue)> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = &mut (String, JsonValue)> {
        self.0.iter_mut()
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.0
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonValue> {
        self.0
            .iter_mut()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value)
    }

    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: String, value: JsonValue) -> Option<JsonValue> {
        if let Some((_, existing)) = self.0.iter_mut().find(|(candidate, _)| candidate == &key) {
            return Some(core::mem::replace(existing, value));
        }
        self.0.push((key, value));
        None
    }

    pub fn remove(&mut self, key: &str) -> Option<JsonValue> {
        self.0
            .iter()
            .position(|(candidate, _)| candidate == key)
            .map(|index| self.0.remove(index).1)
    }

    pub fn append(&mut self, other: &mut Self) {
        self.0.append(&mut other.0);
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&String, &mut JsonValue) -> bool,
    {
        let mut i = 0;
        while i < self.0.len() {
            let keep = {
                let (key, value) = &mut self.0[i];
                f(key, value)
            };
            if keep {
                i += 1;
            } else {
                self.0.remove(i);
            }
        }
    }

    pub fn swap_remove(&mut self, key: &str) -> Option<JsonValue> {
        self.0
            .iter()
            .position(|(candidate, _)| candidate == key)
            .map(|index| self.0.swap_remove(index).1)
    }

    pub fn shift_insert(
        &mut self,
        index: usize,
        key: String,
        value: JsonValue,
    ) -> Option<JsonValue> {
        if let Some((_, existing)) = self.0.iter_mut().find(|(candidate, _)| candidate == &key) {
            return Some(core::mem::replace(existing, value));
        }
        let index = index.min(self.0.len());
        self.0.insert(index, (key, value));
        None
    }

    pub fn sort_keys(&mut self) {
        self.0.sort_by(|(a, _), (b, _)| a.cmp(b));
    }

    pub fn get_or_insert_null(&mut self, key: &str) -> &mut JsonValue {
        if let Some(pos) = self.0.iter().position(|(candidate, _)| candidate == key) {
            &mut self.0[pos].1
        } else {
            self.0.push((key.to_owned(), JsonValue::Null));
            &mut self.0.last_mut().unwrap().1
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn into_vec(self) -> Vec<(String, JsonValue)> {
        self.0
    }

    #[cfg(feature = "serde")]
    pub(crate) fn push_entry(&mut self, entry: (String, JsonValue)) {
        self.0.push(entry);
    }
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<(String, JsonValue)>> for Map {
    fn from(value: Vec<(String, JsonValue)>) -> Self {
        Self(value)
    }
}

impl core::iter::FromIterator<(String, JsonValue)> for Map {
    fn from_iter<T: IntoIterator<Item = (String, JsonValue)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Deref for Map {
    type Target = Vec<(String, JsonValue)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
