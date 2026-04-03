use crate::map::Map;
use crate::number::JsonNumber;
use crate::util;
use crate::ValueIndex;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<JsonValue>),
    Object(Map),
}

pub type Value = JsonValue;
pub type Number = JsonNumber;

impl Default for JsonValue {
    fn default() -> Self {
        Self::Null
    }
}

impl Eq for JsonValue {}

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
        Self::Number(JsonNumber::from(value as i64))
    }
}

impl From<i16> for JsonValue {
    fn from(value: i16) -> Self {
        Self::Number(JsonNumber::from(value as i64))
    }
}

impl From<i32> for JsonValue {
    fn from(value: i32) -> Self {
        Self::Number(JsonNumber::from(value as i64))
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
        Self::Number(JsonNumber::U64(value as u64))
    }
}

impl From<u16> for JsonValue {
    fn from(value: u16) -> Self {
        Self::Number(JsonNumber::U64(value as u64))
    }
}

impl From<u32> for JsonValue {
    fn from(value: u32) -> Self {
        Self::Number(JsonNumber::U64(value as u64))
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
        Self::Number(JsonNumber::F64(value as f64))
    }
}

impl From<f64> for JsonValue {
    fn from(value: f64) -> Self {
        Self::Number(JsonNumber::F64(value))
    }
}

impl From<i128> for JsonValue {
    fn from(value: i128) -> Self {
        JsonNumber::from_i128(value)
            .map(Self::Number)
            .unwrap_or_else(|| Self::String(value.to_string()))
    }
}

impl From<u128> for JsonValue {
    fn from(value: u128) -> Self {
        JsonNumber::from_u128(value)
            .map(Self::Number)
            .unwrap_or_else(|| Self::String(value.to_string()))
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

impl<K, V> std::iter::FromIterator<(K, V)> for JsonValue
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

impl<T> std::iter::FromIterator<T> for JsonValue
where
    T: Into<JsonValue>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::Array(iter.into_iter().map(Into::into).collect())
    }
}
