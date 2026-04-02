use crate::{JsonNumber, JsonValue, Map, Parser};
use crate::serde_error::json_parse_error_to_serde;
use serde_crate::de::{
    value::{BorrowedStrDeserializer, StringDeserializer},
    DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde_crate::{Deserialize, Deserializer as SerdeDeserializer};
use std::borrow::Cow;
use std::marker::PhantomData;

pub struct Deserializer<'de> {
    input: Cow<'de, str>,
    offset: usize,
    failed: bool,
    error: Option<crate::serde_error::Error>,
}

impl Deserializer<'static> {
    pub fn from_reader<R: std::io::Read>(mut reader: R) -> Self {
        let mut input = String::new();
        match reader.read_to_string(&mut input) {
            Ok(_) => Self {
                input: Cow::Owned(input),
                offset: 0,
                failed: false,
                error: None,
            },
            Err(_) => Self {
                input: Cow::Owned(String::new()),
                offset: 0,
                failed: true,
                error: Some(crate::serde_error::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "input is not valid UTF-8",
                ))),
            },
        }
    }
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Self {
            input: Cow::Borrowed(input),
            offset: 0,
            failed: false,
            error: None,
        }
    }

    pub fn from_slice(input: &'de [u8]) -> Self {
        match std::str::from_utf8(input) {
            Ok(text) => Self::from_str(text),
            Err(_) => Self {
                input: Cow::Borrowed(""),
                offset: 0,
                failed: true,
                error: Some(json_parse_error_to_serde("", crate::JsonParseError::InvalidUtf8)),
            },
        }
    }

    pub fn end(&mut self) -> Result<(), crate::serde_error::Error> {
        let mut parser = Parser::new(self.remaining_input());
        parser.skip_whitespace();
        if parser.is_eof() {
            Ok(())
        } else {
            Err(json_parse_error_to_serde(
                self.remaining_input(),
                crate::JsonParseError::UnexpectedTrailingCharacters(parser.index),
            ))
        }
    }

    fn remaining_input(&self) -> &str {
        &self.input[self.offset..]
    }

    fn parse_next_value(&mut self) -> Result<JsonValue, crate::serde_error::Error> {
        if self.failed {
            return Err(self
                .error
                .clone()
                .unwrap_or_else(|| crate::serde_error::Error::custom("deserializer is in a failed state")));
        }

        let remaining = self.remaining_input();
        let mut parser = Parser::new(remaining);
        let value = parser
            .parse_value()
            .map_err(|error| json_parse_error_to_serde(remaining, error))?;
        parser.skip_whitespace();
        self.offset += parser.index;
        Ok(value)
    }
}

impl<'de> SerdeDeserializer<'de> for &mut Deserializer<'de> {
    type Error = crate::serde_error::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_next_value()?;
        JsonValueDeserializer::new(value).deserialize_any(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_next_value()?;
        JsonValueDeserializer::new(value).deserialize_option(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_next_value()?;
        JsonValueDeserializer::new(value).deserialize_enum(name, variants, visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        #[cfg(feature = "raw_value")]
        if name == crate::raw::RAW_VALUE_TOKEN {
            let raw = self.parse_next_value()?;
            let raw_str = raw.to_json_string().map_err(crate::serde_error::json_error_to_serde)?;
            return visitor.visit_map(crate::raw::OwnedRawDeserializer::new(raw_str));
        }
        let value = self.parse_next_value()?;
        JsonValueDeserializer::new(value).deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_next_value()?;
        JsonValueDeserializer::new(value).deserialize_struct(name, fields, visitor)
    }

    serde_crate::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map identifier ignored_any
    }
}

pub struct JsonValueDeserializer {
    value: JsonValue,
}

impl JsonValueDeserializer {
    pub fn new(value: JsonValue) -> Self {
        Self { value }
    }

    fn invalid_type(expected: &str, found: &JsonValue) -> crate::serde_error::Error {
        crate::serde_error::Error::custom(format!(
            "invalid type: expected {}, found {}",
            expected,
            match found {
                JsonValue::Null => "null",
                JsonValue::Bool(_) => "bool",
                JsonValue::Number(_) => "number",
                JsonValue::String(_) => "string",
                JsonValue::Array(_) => "array",
                JsonValue::Object(_) => "object",
            }
        ))
    }
}

impl<'de> SerdeDeserializer<'de> for JsonValueDeserializer {
    type Error = crate::serde_error::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::Null => visitor.visit_unit(),
            JsonValue::Bool(b) => visitor.visit_bool(b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    visitor.visit_i64(i)
                } else if let Some(u) = n.as_u64() {
                    visitor.visit_u64(u)
                } else if let Some(f) = n.as_f64() {
                    visitor.visit_f64(f)
                } else {
                    visitor.visit_str(&n.to_string())
                }
            }
            JsonValue::String(s) => visitor.visit_string(s),
            JsonValue::Array(arr) => {
                let len = arr.len();
                let mut seq = JsonSeqAccess {
                    iter: arr.into_iter(),
                    len,
                };
                visitor.visit_seq(&mut seq)
            }
            JsonValue::Object(obj) => {
                let map: Map = obj;
                let len = map.0.len();
                let seq: Vec<(String, JsonValue)> = map.0.into_iter().collect();
                let mut map_access = JsonMapAccess {
                    iter: seq.into_iter(),
                    len,
                    pending_value: None,
                };
                visitor.visit_map(&mut map_access)
            }
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::Bool(b) => visitor.visit_bool(b),
            other => Err(Self::invalid_type("bool", &other)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::String(s) => visitor.visit_string(s),
            other => Err(Self::invalid_type("string", &other)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::String(s) => visitor.visit_string(s),
            JsonValue::Array(arr) => {
                let bytes: Vec<u8> = arr
                    .into_iter()
                    .map(|v| match v {
                        JsonValue::Number(n) => n.as_u64().unwrap_or(0) as u8,
                        _ => 0,
                    })
                    .collect();
                visitor.visit_byte_buf(bytes)
            }
            other => Err(Self::invalid_type("bytes", &other)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::Null => visitor.visit_unit(),
            other => Err(Self::invalid_type("unit", &other)),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::Array(arr) => {
                let len = arr.len();
                let mut seq = JsonSeqAccess {
                    iter: arr.into_iter(),
                    len,
                };
                visitor.visit_seq(&mut seq)
            }
            other => Err(Self::invalid_type("seq", &other)),
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::Object(obj) => {
                let map: Map = obj;
                let len = map.0.len();
                let seq: Vec<(String, JsonValue)> = map.0.into_iter().collect();
                let mut map_access = JsonMapAccess {
                    iter: seq.into_iter(),
                    len,
                    pending_value: None,
                };
                visitor.visit_map(&mut map_access)
            }
            other => Err(Self::invalid_type("map", &other)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::Object(obj) => {
                let mut iter = obj.iter();
                if let Some((key, value)) = iter.next() {
                    let seed = EnumVariantAccess {
                        variant: key.clone(),
                        value: value.clone(),
                    };
                    return visitor.visit_enum(seed);
                }
                Err(Self::invalid_type("enum", &JsonValue::Null))
            }
            JsonValue::String(variant) => {
                let seed = UnitVariantAccess { variant };
                visitor.visit_enum(seed)
            }
            other => Err(Self::invalid_type("enum", &other)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de> serde_crate::Deserialize<'de> for JsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_crate::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = JsonValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonValue::Number(JsonNumber::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonValue::Number(JsonNumber::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonValue::Number(JsonNumber::from_f64(value).unwrap_or(JsonNumber::F64(value))))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonValue::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonValue::String(value))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut arr = Vec::new();
        while let Some(value) = seq.next_element()? {
            arr.push(value);
        }
        Ok(JsonValue::Array(arr))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut obj = Map::new();
        while let Some((key, value)) = map.next_entry()? {
            obj.0.push((key, value));
        }
        Ok(JsonValue::Object(obj))
    }
}

impl<'de> serde_crate::Deserialize<'de> for JsonNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_crate::Deserializer<'de>,
    {
        deserializer.deserialize_any(NumberVisitor)
    }
}

struct NumberVisitor;

impl<'de> Visitor<'de> for NumberVisitor {
    type Value = JsonNumber;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a JSON number")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonNumber::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonNumber::from(value))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        JsonNumber::from_f64(value).ok_or_else(|| {
            E::custom("invalid floating point number")
        })
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        // Try parsing as f64 first, then i64, then u64
        if let Ok(n) = value.parse::<f64>() {
            if n.is_finite() {
                return Ok(JsonNumber::F64(n));
            }
        }
        if let Ok(n) = value.parse::<i64>() {
            return Ok(JsonNumber::from(n));
        }
        if let Ok(n) = value.parse::<u64>() {
            return Ok(JsonNumber::from(n));
        }
        Err(E::custom("invalid number"))
    }
}

impl<'de> serde_crate::Deserialize<'de> for Map {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde_crate::Deserializer<'de>,
    {
        deserializer.deserialize_map(MapVisitor)
    }
}

struct MapVisitor;

impl<'de> Visitor<'de> for MapVisitor {
    type Value = Map;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut obj = Map::new();
        while let Some((key, value)) = map.next_entry()? {
            obj.0.push((key, value));
        }
        Ok(obj)
    }
}

struct JsonSeqAccess {
    iter: std::vec::IntoIter<JsonValue>,
    len: usize,
}

impl<'de> SeqAccess<'de> for JsonSeqAccess {
    type Error = crate::serde_error::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed
                .deserialize(JsonValueDeserializer::new(value))
                .map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct JsonMapAccess {
    iter: std::vec::IntoIter<(String, JsonValue)>,
    len: usize,
    pending_value: Option<JsonValue>,
}

impl<'de> MapAccess<'de> for JsonMapAccess {
    type Error = crate::serde_error::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.pending_value = Some(value);
                seed.deserialize(StringDeserializer::new(key)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self.pending_value.take().expect("next_value_seed called before next_key_seed");
        seed.deserialize(JsonValueDeserializer::new(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct EnumVariantAccess {
    variant: String,
    value: JsonValue,
}

impl<'de> EnumAccess<'de> for EnumVariantAccess {
    type Error = crate::serde_error::Error;
    type Variant = JsonValueDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let deserializer: StringDeserializer<crate::serde_error::Error> = StringDeserializer::new(self.variant);
        let variant = seed.deserialize(deserializer)?;
        Ok((variant, JsonValueDeserializer::new(self.value)))
    }
}

impl<'de> VariantAccess<'de> for JsonValueDeserializer {
    type Error = crate::serde_error::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_struct("", fields, visitor)
    }
}

struct UnitVariantAccess {
    variant: String,
}

impl<'de> EnumAccess<'de> for UnitVariantAccess {
    type Error = crate::serde_error::Error;
    type Variant = Impossible<(), crate::serde_error::Error>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let deserializer: StringDeserializer<crate::serde_error::Error> = StringDeserializer::new(self.variant);
        let variant = seed.deserialize(deserializer)?;
        Ok((variant, Impossible(PhantomData, PhantomData)))
    }
}

struct Impossible<T, E>(PhantomData<fn() -> T>, PhantomData<E>);

impl<'de, T, E: serde_crate::de::Error> VariantAccess<'de> for Impossible<T, E> {
    type Error = E;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<U>(self, _: U) -> Result<U::Value, Self::Error>
    where
        U: DeserializeSeed<'de>,
    {
        unreachable!()
    }

    fn tuple_variant<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unreachable!()
    }
}