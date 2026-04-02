use crate::error::{JsonError, JsonParseError};
use crate::parse::Parser;
use crate::tape::{BorrowedJsonValue, TapeToken, TapeTokenKind};
use core::fmt;
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io::Write;
use std::ops::{Deref, DerefMut, Index, IndexMut};

#[derive(Clone, Debug, PartialEq)]
pub enum JsonNumber {
    I64(i64),
    U64(u64),
    F64(f64),
}

impl JsonNumber {
    pub fn from_i128(value: i128) -> Option<Self> {
        if let Ok(value) = u64::try_from(value) {
            Some(Self::U64(value))
        } else if let Ok(value) = i64::try_from(value) {
            Some(Self::I64(value))
        } else {
            None
        }
    }

    pub fn from_u128(value: u128) -> Option<Self> {
        u64::try_from(value).ok().map(Self::U64)
    }

    pub fn is_i64(&self) -> bool {
        match self {
            Self::I64(_) => true,
            Self::U64(value) => *value <= i64::MAX as u64,
            Self::F64(_) => false,
        }
    }

    pub fn is_u64(&self) -> bool {
        matches!(self, Self::U64(_))
    }

    pub fn is_f64(&self) -> bool {
        matches!(self, Self::F64(_))
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(value) => Some(*value),
            Self::U64(value) => (*value <= i64::MAX as u64).then_some(*value as i64),
            Self::F64(_) => None,
        }
    }

    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Self::I64(value) => Some(*value as i128),
            Self::U64(value) => Some(*value as i128),
            Self::F64(_) => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::I64(value) => (*value >= 0).then_some(*value as u64),
            Self::U64(value) => Some(*value),
            Self::F64(_) => None,
        }
    }

    pub fn as_u128(&self) -> Option<u128> {
        match self {
            Self::I64(value) => (*value >= 0).then_some(*value as u128),
            Self::U64(value) => Some(*value as u128),
            Self::F64(_) => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::I64(value) => Some(*value as f64),
            Self::U64(value) => Some(*value as f64),
            Self::F64(value) => Some(*value),
        }
    }

    pub fn from_f64(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self::F64(value))
    }

    pub fn from_string_unchecked(n: String) -> Self {
        match Parser::new(&n).parse_value() {
            Ok(JsonValue::Number(number)) => number,
            _ => panic!("from_string_unchecked called with non-number JSON"),
        }
    }
}

impl Display for JsonNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I64(value) => write!(f, "{value}"),
            Self::U64(value) => write!(f, "{value}"),
            Self::F64(value) => write!(f, "{value}"),
        }
    }
}

impl From<i64> for JsonNumber {
    fn from(value: i64) -> Self {
        if value >= 0 {
            Self::U64(value as u64)
        } else {
            Self::I64(value)
        }
    }
}

impl From<u64> for JsonNumber {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<JsonValue>),
    Object(Map),
}

impl Default for JsonValue {
    fn default() -> Self {
        Self::Null
    }
}

impl<'a> Default for &'a JsonValue {
    fn default() -> Self {
        &JSON_NULL
    }
}

impl Eq for JsonValue {}

pub type Value = JsonValue;
pub type Number = JsonNumber;
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(feature = "serde")]
pub type Error = crate::serde_error::Error;

#[cfg(not(feature = "serde"))]
pub type Error = JsonError;

#[derive(Clone, Debug, PartialEq)]
pub struct Map<K = String, V = JsonValue>(
    pub Vec<(String, JsonValue)>,
    pub std::marker::PhantomData<(K, V)>,
);

impl Map {
    pub fn new() -> Self {
        Self(Vec::new(), std::marker::PhantomData)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity), std::marker::PhantomData)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn keys(&self) -> crate::Keys<'_> {
        crate::Keys(self.0.iter())
    }

    pub fn values(&self) -> crate::Values<'_> {
        crate::Values(self.0.iter())
    }

    pub fn values_mut(&mut self) -> crate::ValuesMut<'_> {
        crate::ValuesMut(self.0.iter_mut())
    }

    pub fn iter(&self) -> crate::Iter<'_> {
        crate::Iter(self.0.iter())
    }

    pub fn iter_mut(&mut self) -> crate::IterMut<'_> {
        crate::IterMut(self.0.iter_mut())
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&JsonValue>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.0
            .iter()
            .find(|(candidate, _)| <String as std::borrow::Borrow<Q>>::borrow(candidate) == key)
            .map(|(_, value)| value)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut JsonValue>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.0
            .iter_mut()
            .find(|(candidate, _)| <String as std::borrow::Borrow<Q>>::borrow(candidate) == key)
            .map(|(_, value)| value)
    }

    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&String, &JsonValue)>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.0
            .iter()
            .find(|(candidate, _)| <String as std::borrow::Borrow<Q>>::borrow(candidate) == key)
            .map(|(k, v)| (k, v))
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.get(key).is_some()
    }

    pub fn insert(&mut self, key: String, value: JsonValue) -> Option<JsonValue> {
        if let Some((_, existing)) = self.0.iter_mut().find(|(candidate, _)| candidate == &key) {
            return Some(std::mem::replace(existing, value));
        }
        self.0.push((key, value));
        None
    }

    pub fn entry<S>(&mut self, key: S) -> crate::MapEntry<'_>
    where
        S: Into<String>,
    {
        let key = key.into();
        if let Some(index) = self.0.iter().position(|(candidate, _)| candidate == &key) {
            crate::MapEntry::Occupied(crate::OccupiedEntry { map: self, index })
        } else {
            crate::MapEntry::Vacant(crate::VacantEntry { map: self, key })
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<JsonValue>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.0
            .iter()
            .position(|(candidate, _)| <String as std::borrow::Borrow<Q>>::borrow(candidate) == key)
            .map(|index| self.0.remove(index).1)
    }

    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(String, JsonValue)>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.0
            .iter()
            .position(|(candidate, _)| <String as std::borrow::Borrow<Q>>::borrow(candidate) == key)
            .map(|index| self.0.remove(index))
    }

    pub fn shift_insert(
        &mut self,
        index: usize,
        key: String,
        value: JsonValue,
    ) -> Option<JsonValue> {
        if let Some(existing_index) = self.0.iter().position(|(candidate, _)| candidate == &key) {
            let (_, old_value) = self.0.remove(existing_index);
            let target = if existing_index < index {
                index.saturating_sub(1)
            } else {
                index
            };
            let insert_at = target.min(self.0.len());
            self.0.insert(insert_at, (key, value));
            return Some(old_value);
        }
        let insert_at = index.min(self.0.len());
        self.0.insert(insert_at, (key, value));
        None
    }

    pub fn shift_remove<Q>(&mut self, key: &Q) -> Option<JsonValue>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.remove(key)
    }

    pub fn shift_remove_entry<Q>(&mut self, key: &Q) -> Option<(String, JsonValue)>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.remove_entry(key)
    }

    pub fn swap_remove<Q>(&mut self, key: &Q) -> Option<JsonValue>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.0
            .iter()
            .position(|(candidate, _)| <String as std::borrow::Borrow<Q>>::borrow(candidate) == key)
            .map(|index| self.0.swap_remove(index).1)
    }

    pub fn swap_remove_entry<Q>(&mut self, key: &Q) -> Option<(String, JsonValue)>
    where
        String: std::borrow::Borrow<Q>,
        Q: ?Sized + Ord + Eq + Hash,
    {
        self.0
            .iter()
            .position(|(candidate, _)| <String as std::borrow::Borrow<Q>>::borrow(candidate) == key)
            .map(|index| self.0.swap_remove(index))
    }

    pub fn sort_keys(&mut self) {
        self.0.sort_by(|a, b| a.0.cmp(&b.0));
    }

    pub fn append(&mut self, other: &mut Self) {
        self.0.append(&mut other.0);
    }

    pub fn into_values(self) -> crate::IntoValues {
        crate::IntoValues(self.0.into_iter())
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
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<(String, JsonValue)>> for Map {
    fn from(value: Vec<(String, JsonValue)>) -> Self {
        Self(value, std::marker::PhantomData)
    }
}

impl std::iter::FromIterator<(String, JsonValue)> for Map {
    fn from_iter<T: IntoIterator<Item = (String, JsonValue)>>(iter: T) -> Self {
        Self(iter.into_iter().collect(), std::marker::PhantomData)
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

impl IntoIterator for Map {
    type Item = (String, JsonValue);
    type IntoIter = crate::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        crate::IntoIter(self.0.into_iter())
    }
}

impl<'a> IntoIterator for &'a Map {
    type Item = (&'a String, &'a JsonValue);
    type IntoIter = crate::Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Map {
    type Item = (&'a String, &'a mut JsonValue);
    type IntoIter = crate::IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl JsonValue {
    pub fn object(entries: Vec<(impl Into<String>, JsonValue)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect::<Vec<_>>()
                .into(),
        )
    }

    pub fn array(values: Vec<JsonValue>) -> Self {
        Self::Array(values)
    }

    pub fn to_json_string(&self) -> Result<String, JsonError> {
        let mut out = Vec::with_capacity(crate::util::initial_json_capacity(self));
        crate::util::write_json_value(&mut out, self)?;
        Ok(unsafe { String::from_utf8_unchecked(out) })
    }

    pub fn push_field(&mut self, key: impl Into<String>, value: impl Into<JsonValue>) {
        match self {
            Self::Object(entries) => entries.push((key.into(), value.into())),
            _ => panic!("push_field called on non-object JSON value"),
        }
    }

    pub fn push_item(&mut self, value: impl Into<JsonValue>) {
        match self {
            Self::Array(values) => values.push(value.into()),
            _ => panic!("push_item called on non-array JSON value"),
        }
    }

    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    pub fn as_null(&self) -> Option<()> {
        matches!(self, Self::Null).then_some(())
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<&JsonNumber> {
        match self {
            Self::Number(number) => Some(number),
            _ => None,
        }
    }

    pub fn is_i64(&self) -> bool {
        self.as_number().is_some_and(JsonNumber::is_i64)
    }

    pub fn is_u64(&self) -> bool {
        self.as_number().is_some_and(JsonNumber::is_u64)
    }

    pub fn is_f64(&self) -> bool {
        self.as_number().is_some_and(JsonNumber::is_f64)
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_number().and_then(JsonNumber::as_i64)
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_number().and_then(JsonNumber::as_u64)
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_number().and_then(JsonNumber::as_f64)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<JsonValue>> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Map> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Map> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn get<I>(&self, index: I) -> Option<&JsonValue>
    where
        I: crate::ValueIndex,
    {
        index.index_into(self)
    }

    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut JsonValue>
    where
        I: crate::ValueIndex,
    {
        index.index_into_mut(self)
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Array(values) => values.len(),
            Self::Object(entries) => entries.len(),
            _ => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_i128(&self) -> Option<i128> {
        self.as_i64().map(|v| v as i128)
    }

    pub fn as_u128(&self) -> Option<u128> {
        self.as_u64().map(|v| v as u128)
    }

    pub fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|v| v as f32)
    }

    pub fn get_index(&self, index: usize) -> Option<&JsonValue> {
        match self {
            Self::Array(values) => values.get(index),
            _ => None,
        }
    }

    pub fn get_index_mut(&mut self, index: usize) -> Option<&mut JsonValue> {
        match self {
            Self::Array(values) => values.get_mut(index),
            _ => None,
        }
    }

    pub fn take(&mut self) -> JsonValue {
        std::mem::replace(self, JsonValue::Null)
    }

    pub fn pointer(&self, pointer: &str) -> Option<&JsonValue> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let mut current = self;
        for segment in pointer.split('/').skip(1) {
            let token = crate::util::decode_pointer_segment(segment);
            current = match current {
                JsonValue::Object(entries) => entries
                    .iter()
                    .find(|(key, _)| key.as_str() == token)
                    .map(|(_, value)| value)?,
                JsonValue::Array(values) => values.get(token.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    pub fn pointer_mut(&mut self, pointer: &str) -> Option<&mut JsonValue> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        let mut current = self;
        for segment in pointer.split('/').skip(1) {
            let token = crate::util::decode_pointer_segment(segment);
            current = match current {
                JsonValue::Object(entries) => entries
                    .iter_mut()
                    .find(|(key, _)| key.as_str() == token)
                    .map(|(_, value)| value)?,
                JsonValue::Array(values) => values.get_mut(token.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    pub fn sort_all_objects(&mut self) {
        match self {
            JsonValue::Object(entries) => {
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                for (_, value) in entries.iter_mut() {
                    value.sort_all_objects();
                }
            }
            JsonValue::Array(values) => {
                for value in values.iter_mut() {
                    value.sort_all_objects();
                }
            }
            _ => {}
        }
    }
}

impl Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_json_string() {
            Ok(json) => f.write_str(&json),
            Err(_) => Err(fmt::Error),
        }
    }
}

static JSON_NULL: JsonValue = JsonValue::Null;

pub type MapEntry<'a> = crate::map::MapEntry<'a>;
pub type OccupiedEntry<'a> = crate::map::OccupiedEntry<'a>;
pub type VacantEntry<'a> = crate::map::VacantEntry<'a>;
