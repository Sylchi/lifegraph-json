//! JSON value types and operations.
//!
//! # Types
//!
//! - [`JsonValue`] — the main owned JSON value type
//! - [`Value`] — alias for [`JsonValue`]
//! - [`JsonNumber`] — JSON number representation (see [`number`](crate::number))
//!
//! # Example
//!
//! ```
//! use lifegraph_json::{json, JsonValue};
//!
//! // Using the json! macro
//! let value = json!({"name": "Alice", "age": 30, "active": true});
//! assert_eq!(value["name"].as_str(), Some("Alice"));
//!
//! // Building programmatically
//! let obj = JsonValue::object(vec![
//!     ("id", 1.into()),
//!     ("tags", JsonValue::array(vec!["a".into(), "b".into()])),
//! ]);
//! assert_eq!(obj["id"].as_u64(), Some(1));
//! ```

#[cfg(not(feature = "std"))]
use alloc::borrow::ToOwned;
#[cfg(not(feature = "std"))]
use alloc::string::ToString;

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::map::Map;
use crate::number::JsonNumber;
use crate::util;
use crate::ValueIndex;
use core::fmt;

/// A JSON value that owns its data.
///
/// This is the main JSON value type in lifegraph-json, supporting all standard
/// JSON types: null, boolean, number, string, array, and object.
///
/// # Construction
///
/// Use the [`json!`] macro for literal-like syntax, or the [`JsonValue::object`]
/// and [`JsonValue::array`] constructors for programmatic building.
///
/// # Example
///
/// ```
/// use lifegraph_json::{json, JsonValue};
///
/// let v = json!({"key": "value", "nums": [1, 2, 3]});
/// assert!(v.is_object());
/// assert_eq!(v["key"].as_str(), Some("value"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub enum JsonValue {
    #[default]
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<JsonValue>),
    Object(Map),
}

/// Type alias for [`JsonValue`].
pub type Value = JsonValue;
/// Type alias for [`JsonNumber`].
pub type Number = JsonNumber;

impl Eq for JsonValue {}

impl JsonValue {
    /// Creates a JSON object from key-value pairs.
    ///
    /// # Example
    ///
    /// ```
    /// use lifegraph_json::JsonValue;
    ///
    /// let obj = JsonValue::object(vec![
    ///     ("name", "Alice".into()),
    ///     ("age", 30.into()),
    /// ]);
    /// assert_eq!(obj["name"].as_str(), Some("Alice"));
    /// ```
    #[must_use]
    pub fn object(entries: Vec<(impl Into<String>, JsonValue)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect::<Vec<_>>()
                .into(),
        )
    }

    /// Creates a JSON array from values.
    ///
    /// # Example
    ///
    /// ```
    /// use lifegraph_json::JsonValue;
    ///
    /// let arr = JsonValue::array(vec![1.into(), 2.into(), 3.into()]);
    /// assert_eq!(arr.len(), 3);
    /// assert_eq!(arr[0].as_u64(), Some(1));
    /// ```
    #[must_use]
    pub fn array(values: Vec<JsonValue>) -> Self {
        Self::Array(values)
    }

    /// Serializes this JSON value to a compact JSON string.
    ///
    /// # Example
    ///
    /// ```
    /// use lifegraph_json::JsonValue;
    ///
    /// let value = JsonValue::object(vec![("ok", true.into())]);
    /// assert_eq!(value.to_json_string().unwrap(), r#"{"ok":true}"#);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`JsonError::NonFiniteNumber`] if the value contains `NaN` or `Infinity`.
    pub fn to_json_string(&self) -> Result<String, crate::error::JsonError> {
        let mut out = Vec::with_capacity(util::initial_json_capacity(self));
        util::write_json_value(&mut out, self)?;
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

    #[must_use]
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    #[must_use]
    pub fn as_null(&self) -> Option<()> {
        matches!(self, Self::Null).then_some(())
    }

    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    #[must_use]
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    #[must_use]
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
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

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.as_str()),
            _ => None,
        }
    }

    #[must_use]
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

    #[must_use]
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
        I: ValueIndex,
    {
        index.index_into(self)
    }

    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut JsonValue>
    where
        I: ValueIndex,
    {
        index.index_into_mut(self)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Array(values) => values.len(),
            Self::Object(entries) => entries.len(),
            _ => 0,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn as_i128(&self) -> Option<i128> {
        self.as_i64().map(i128::from)
    }

    #[must_use]
    pub fn as_u128(&self) -> Option<u128> {
        self.as_u64().map(u128::from)
    }

    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|v| v as f32)
    }

    #[must_use]
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
        core::mem::replace(self, JsonValue::Null)
    }

    #[must_use]
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

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_json_string() {
            Ok(json) => f.write_str(&json),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<i8> for JsonValue {
    fn from(value: i8) -> Self {
        Self::Number(JsonNumber::from(i64::from(value)))
    }
}

impl From<i16> for JsonValue {
    fn from(value: i16) -> Self {
        Self::Number(JsonNumber::from(i64::from(value)))
    }
}

impl From<i32> for JsonValue {
    fn from(value: i32) -> Self {
        Self::Number(JsonNumber::from(i64::from(value)))
    }
}

impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        Self::Number(JsonNumber::from(value))
    }
}

impl From<isize> for JsonValue {
    fn from(value: isize) -> Self {
        Self::Number(JsonNumber::from(value as i64))
    }
}

impl From<u8> for JsonValue {
    fn from(value: u8) -> Self {
        Self::Number(JsonNumber::U64(u64::from(value)))
    }
}

impl From<u16> for JsonValue {
    fn from(value: u16) -> Self {
        Self::Number(JsonNumber::U64(u64::from(value)))
    }
}

impl From<u32> for JsonValue {
    fn from(value: u32) -> Self {
        Self::Number(JsonNumber::U64(u64::from(value)))
    }
}

impl From<u64> for JsonValue {
    fn from(value: u64) -> Self {
        Self::Number(JsonNumber::U64(value))
    }
}

impl From<usize> for JsonValue {
    fn from(value: usize) -> Self {
        Self::Number(JsonNumber::U64(value as u64))
    }
}

impl From<f32> for JsonValue {
    fn from(value: f32) -> Self {
        Self::Number(JsonNumber::F64(f64::from(value)))
    }
}

impl From<f64> for JsonValue {
    fn from(value: f64) -> Self {
        Self::Number(JsonNumber::F64(value))
    }
}

impl From<i128> for JsonValue {
    fn from(value: i128) -> Self {
        JsonNumber::from_i128(value).map_or_else(|| Self::String(value.to_string()), Self::Number)
    }
}

impl From<u128> for JsonValue {
    fn from(value: u128) -> Self {
        JsonNumber::from_u128(value).map_or_else(|| Self::String(value.to_string()), Self::Number)
    }
}

impl<T> From<Option<T>> for JsonValue
where
    T: Into<JsonValue>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.into(),
            None => Self::Null,
        }
    }
}

impl<T> From<Vec<T>> for JsonValue
where
    T: Into<JsonValue>,
{
    fn from(values: Vec<T>) -> Self {
        Self::Array(values.into_iter().map(Into::into).collect())
    }
}

impl<K, V> core::iter::FromIterator<(K, V)> for JsonValue
where
    K: Into<String>,
    V: Into<JsonValue>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::Object(
            iter.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect::<Vec<_>>()
                .into(),
        )
    }
}

impl<T> core::iter::FromIterator<T> for JsonValue
where
    T: Into<JsonValue>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::Array(iter.into_iter().map(Into::into).collect())
    }
}
