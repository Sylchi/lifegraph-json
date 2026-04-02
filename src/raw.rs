use crate::serde_error::Error;
use crate::{JsonParseError, JsonValue};
use serde_crate::de::{
    value::StringDeserializer, DeserializeSeed, Deserializer as SerdeDeserializer, MapAccess,
    Visitor,
};
use serde_crate::ser::SerializeStruct;
use serde_crate::{Deserialize, Serialize};
use std::borrow::Cow;

pub const RAW_VALUE_TOKEN: &str = "$serde_json::private::RawValue";

#[repr(transparent)]
pub struct RawValue(str);

impl RawValue {
    pub fn from_string(json: String) -> Result<Box<Self>, JsonParseError> {
        crate::parse_json(&json)?;
        let boxed = json.into_boxed_str();
        Ok(unsafe { Box::from_raw(Box::into_raw(boxed) as *mut RawValue) })
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for RawValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RawValue").field(&self.get()).finish()
    }
}

impl std::fmt::Display for RawValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get())
    }
}

impl Clone for Box<RawValue> {
    fn clone(&self) -> Self {
        let boxed = self.get().to_owned().into_boxed_str();
        unsafe { Box::from_raw(Box::into_raw(boxed) as *mut RawValue) }
    }
}

impl ToOwned for RawValue {
    type Owned = Box<RawValue>;

    fn to_owned(&self) -> Self::Owned {
        let boxed = self.get().to_owned().into_boxed_str();
        unsafe { Box::from_raw(Box::into_raw(boxed) as *mut RawValue) }
    }
}

impl From<Box<RawValue>> for Box<str> {
    fn from(raw_value: Box<RawValue>) -> Self {
        unsafe { Box::from_raw(Box::into_raw(raw_value) as *mut str) }
    }
}

pub struct OwnedRawDeserializer {
    raw_value: Option<String>,
}

impl OwnedRawDeserializer {
    pub fn new(raw_value: String) -> Self {
        Self {
            raw_value: Some(raw_value),
        }
    }
}

impl Default for Box<RawValue> {
    fn default() -> Self {
        RawValue::from_string("null".to_owned()).expect("null is always valid JSON")
    }
}

pub struct BorrowedRawDeserializer<'de> {
    raw_value: Option<&'de str>,
}

impl<'de> BorrowedRawDeserializer<'de> {
    pub fn new(raw_value: &'de str) -> Self {
        Self {
            raw_value: Some(raw_value),
        }
    }
}

struct RawKey;

impl<'de> Deserialize<'de> for RawKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: SerdeDeserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("raw value")
            }

            fn visit_str<E>(self, s: &str) -> std::result::Result<(), E>
            where
                E: serde_crate::de::Error,
            {
                if s == RAW_VALUE_TOKEN {
                    Ok(())
                } else {
                    Err(E::custom("unexpected raw value"))
                }
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)?;
        Ok(RawKey)
    }
}

struct RawKeyDeserializer;

impl<'de> SerdeDeserializer<'de> for RawKeyDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(RAW_VALUE_TOKEN)
    }

    serde_crate::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 u128 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct ignored_any
        unit_struct tuple_struct tuple enum identifier
    }
}

pub struct ReferenceFromString;

impl<'de> DeserializeSeed<'de> for ReferenceFromString {
    type Value = &'de RawValue;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: SerdeDeserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for ReferenceFromString {
    type Value = &'de RawValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("raw value")
    }

    fn visit_borrowed_str<E>(self, s: &'de str) -> std::result::Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(unsafe { &*(s as *const str as *const RawValue) })
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        let leaked: &'static str = Box::leak(s.to_owned().into_boxed_str());
        Ok(unsafe { &*(leaked as *const str as *const RawValue) })
    }

    fn visit_string<E>(self, s: String) -> std::result::Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        let leaked: &'static str = Box::leak(s.into_boxed_str());
        Ok(unsafe { &*(leaked as *const str as *const RawValue) })
    }
}

pub struct BoxedFromString;

impl<'de> DeserializeSeed<'de> for BoxedFromString {
    type Value = Box<RawValue>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: SerdeDeserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for BoxedFromString {
    type Value = Box<RawValue>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("raw value")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        RawValue::from_string(s.to_owned()).map_err(|error| E::custom(error.to_string()))
    }

    fn visit_string<E>(self, s: String) -> std::result::Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        RawValue::from_string(s).map_err(|error| E::custom(error.to_string()))
    }
}

impl<'de> MapAccess<'de> for OwnedRawDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.raw_value.is_none() {
            return Ok(None);
        }
        seed.deserialize(RawKeyDeserializer).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let raw = self
            .raw_value
            .take()
            .ok_or_else(|| Error::custom("raw value missing"))?;
        seed.deserialize(StringDeserializer::<Error>::new(raw))
    }
}

impl<'de> MapAccess<'de> for BorrowedRawDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.raw_value.is_none() {
            return Ok(None);
        }
        seed.deserialize(RawKeyDeserializer).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let raw = self
            .raw_value
            .take()
            .ok_or_else(|| Error::custom("raw value missing"))?;
        seed.deserialize(serde_crate::de::value::BorrowedStrDeserializer::new(raw))
    }
}

impl Serialize for RawValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde_crate::ser::Serializer,
    {
        let mut state = serializer.serialize_struct(RAW_VALUE_TOKEN, 1)?;
        state.serialize_field(RAW_VALUE_TOKEN, self.get())?;
        state.end()
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a RawValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: SerdeDeserializer<'de>,
    {
        struct ReferenceVisitor;

        impl<'de> Visitor<'de> for ReferenceVisitor {
            type Value = &'de RawValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }

            fn visit_map<V>(self, mut visitor: V) -> std::result::Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                if visitor.next_key::<RawKey>()?.is_none() {
                    return Err(<V::Error as serde_crate::de::Error>::custom(
                        "invalid raw value",
                    ));
                }
                visitor.next_value_seed(ReferenceFromString)
            }
        }

        deserializer.deserialize_newtype_struct(RAW_VALUE_TOKEN, ReferenceVisitor)
    }
}

impl<'de> Deserialize<'de> for Box<RawValue> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: SerdeDeserializer<'de>,
    {
        struct BoxedVisitor;

        impl<'de> Visitor<'de> for BoxedVisitor {
            type Value = Box<RawValue>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }

            fn visit_map<V>(self, mut visitor: V) -> std::result::Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                if visitor.next_key::<RawKey>()?.is_none() {
                    return Err(<V::Error as serde_crate::de::Error>::custom(
                        "invalid raw value",
                    ));
                }
                visitor.next_value_seed(BoxedFromString)
            }
        }

        deserializer.deserialize_newtype_struct(RAW_VALUE_TOKEN, BoxedVisitor)
    }
}

pub fn to_raw_value<T>(value: &T) -> Result<Box<RawValue>, crate::serde_error::Error>
where
    T: Serialize + ?Sized,
{
    let json = crate::to_string(value)?;
    RawValue::from_string(json)
        .map_err(|_| crate::serde_error::Error::custom("invalid JSON from serialization"))
}
