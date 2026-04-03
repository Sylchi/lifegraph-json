#[allow(unused_imports)]
#[cfg(not(feature = "std"))]
use alloc::string::String;
#[allow(unused_imports)]
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::map::Map;
use crate::JsonValue;
use core::ops::{Index, IndexMut};

pub trait ValueIndex {
    fn index_into<'a>(&self, value: &'a JsonValue) -> Option<&'a JsonValue>;
    fn index_into_mut<'a>(&self, value: &'a mut JsonValue) -> Option<&'a mut JsonValue>;
}

impl ValueIndex for usize {
    fn index_into<'a>(&self, value: &'a JsonValue) -> Option<&'a JsonValue> {
        match value {
            JsonValue::Array(values) => values.get(*self),
            _ => None,
        }
    }

    fn index_into_mut<'a>(&self, value: &'a mut JsonValue) -> Option<&'a mut JsonValue> {
        match value {
            JsonValue::Array(values) => values.get_mut(*self),
            _ => None,
        }
    }
}

impl ValueIndex for str {
    fn index_into<'a>(&self, value: &'a JsonValue) -> Option<&'a JsonValue> {
        match value {
            JsonValue::Object(entries) => object_get(entries, self),
            _ => None,
        }
    }

    fn index_into_mut<'a>(&self, value: &'a mut JsonValue) -> Option<&'a mut JsonValue> {
        match value {
            JsonValue::Object(entries) => object_get_mut(entries, self),
            _ => None,
        }
    }
}

impl ValueIndex for String {
    fn index_into<'a>(&self, value: &'a JsonValue) -> Option<&'a JsonValue> {
        self.as_str().index_into(value)
    }

    fn index_into_mut<'a>(&self, value: &'a mut JsonValue) -> Option<&'a mut JsonValue> {
        self.as_str().index_into_mut(value)
    }
}

impl<T> ValueIndex for &T
where
    T: ?Sized + ValueIndex,
{
    fn index_into<'a>(&self, value: &'a JsonValue) -> Option<&'a JsonValue> {
        (**self).index_into(value)
    }

    fn index_into_mut<'a>(&self, value: &'a mut JsonValue) -> Option<&'a mut JsonValue> {
        (**self).index_into_mut(value)
    }
}

fn object_get<'a>(entries: &'a Map, key: &str) -> Option<&'a JsonValue> {
    entries.get(key)
}

fn object_get_mut<'a>(entries: &'a mut Map, key: &str) -> Option<&'a mut JsonValue> {
    entries.get_mut(key)
}

pub fn object_index_or_insert<'a>(value: &'a mut JsonValue, key: &str) -> &'a mut JsonValue {
    if matches!(value, JsonValue::Null) {
        *value = JsonValue::Object(Map::new());
    }
    match value {
        JsonValue::Object(entries) => entries.get_or_insert_null(key),
        JsonValue::Null => unreachable!(),
        JsonValue::Bool(_) => panic!("cannot access key {key:?} in JSON boolean"),
        JsonValue::Number(_) => panic!("cannot access key {key:?} in JSON number"),
        JsonValue::String(_) => panic!("cannot access key {key:?} in JSON string"),
        JsonValue::Array(_) => panic!("cannot access key {key:?} in JSON array"),
    }
}

fn array_index_or_panic(value: &mut JsonValue, index: usize) -> &mut JsonValue {
    match value {
        JsonValue::Array(values) => {
            let len = values.len();
            values.get_mut(index).unwrap_or_else(|| {
                panic!("cannot access index {index} of JSON array of length {len}")
            })
        }
        JsonValue::Null => panic!("cannot access index {index} of JSON null"),
        JsonValue::Bool(_) => panic!("cannot access index {index} of JSON boolean"),
        JsonValue::Number(_) => panic!("cannot access index {index} of JSON number"),
        JsonValue::String(_) => panic!("cannot access index {index} of JSON string"),
        JsonValue::Object(_) => panic!("cannot access index {index} of JSON object"),
    }
}

static JSON_NULL: JsonValue = JsonValue::Null;

impl Index<&str> for JsonValue {
    type Output = JsonValue;

    fn index(&self, index: &str) -> &Self::Output {
        match self {
            JsonValue::Object(entries) => object_get(entries, index).unwrap_or(&JSON_NULL),
            _ => &JSON_NULL,
        }
    }
}

impl Index<String> for JsonValue {
    type Output = JsonValue;

    fn index(&self, index: String) -> &Self::Output {
        self.index(index.as_str())
    }
}

impl Index<usize> for JsonValue {
    type Output = JsonValue;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            JsonValue::Array(values) => values.get(index).unwrap_or(&JSON_NULL),
            _ => &JSON_NULL,
        }
    }
}

impl IndexMut<&str> for JsonValue {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        object_index_or_insert(self, index)
    }
}

impl IndexMut<String> for JsonValue {
    fn index_mut(&mut self, index: String) -> &mut Self::Output {
        object_index_or_insert(self, &index)
    }
}

impl IndexMut<usize> for JsonValue {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        array_index_or_panic(self, index)
    }
}
