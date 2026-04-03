#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::map::Map;
use crate::number::JsonNumber;
use crate::parse::Parser;
use crate::serde_error::json_parse_error_to_serde;
use crate::JsonValue;
use serde_crate::de::{
    value::StringDeserializer, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde_crate::Deserializer as SerdeDeserializer;
use std::borrow::Cow;
use std::marker::PhantomData;

/// Single-pass JSON deserializer.
///
/// Drives the parser and serde visitor together — no intermediate `JsonValue` tree.
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
                error: Some(json_parse_error_to_serde(
                    "",
                    crate::JsonParseError::InvalidUtf8,
                )),
            },
        }
    }

    pub fn end(&mut self) -> Result<(), crate::serde_error::Error> {
        if self.failed {
            return Err(self.error.clone().unwrap_or_else(|| {
                crate::serde_error::Error::custom("deserializer is in a failed state")
            }));
        }
        let remaining = self.remaining_input();
        let mut p = Parser::new(remaining);
        p.skip_whitespace();
        if p.is_eof() {
            self.offset = self.input.len();
            Ok(())
        } else {
            Err(json_parse_error_to_serde(
                remaining,
                crate::JsonParseError::UnexpectedTrailingCharacters(p.index()),
            ))
        }
    }

    fn remaining_input(&self) -> &str {
        &self.input[self.offset..]
    }

    fn skip_whitespace(&mut self) {
        let remaining = self.remaining_input();
        let mut i = 0;
        for byte in remaining.bytes() {
            match byte {
                b' ' | b'\n' | b'\r' | b'\t' => i += 1,
                _ => break,
            }
        }
        self.offset += i;
    }

    fn peek_byte(&mut self) -> Option<u8> {
        self.skip_whitespace();
        self.remaining_input().as_bytes().first().copied()
    }

    fn error_literal(index: usize) -> crate::JsonParseError {
        crate::JsonParseError::InvalidLiteral { index }
    }

    fn error_unexpected(index: usize, found: char) -> crate::JsonParseError {
        crate::JsonParseError::UnexpectedCharacter { index, found }
    }

    /// Deserialize the next JSON value directly into the visitor's output type.
    fn deserialize_direct<V>(&mut self, visitor: V) -> Result<V::Value, crate::serde_error::Error>
    where
        V: Visitor<'de>,
    {
        if self.failed {
            return Err(self.error.clone().unwrap_or_else(|| {
                crate::serde_error::Error::custom("deserializer is in a failed state")
            }));
        }
        match self.peek_byte() {
            Some(b'n') => self.consume_literal(b"null", visitor.visit_unit()),
            Some(b't') => self.consume_literal(b"true", visitor.visit_bool(true)),
            Some(b'f') => self.consume_literal(b"false", visitor.visit_bool(false)),
            Some(b'"') => self.consume_string(visitor),
            Some(b'[') => self.consume_array(visitor),
            Some(b'{') => self.consume_object(visitor),
            Some(b'-' | b'0'..=b'9') => self.consume_number(visitor),
            Some(found) => {
                let err = Self::error_unexpected(self.offset, found as char);
                Err(json_parse_error_to_serde(self.remaining_input(), err))
            }
            None => {
                let err = crate::JsonParseError::UnexpectedEnd;
                Err(json_parse_error_to_serde("", err))
            }
        }
    }

    fn consume_literal<T>(
        &mut self,
        expected: &[u8],
        result: Result<T, crate::serde_error::Error>,
    ) -> Result<T, crate::serde_error::Error> {
        if self.remaining_input().as_bytes().starts_with(expected) {
            self.offset += expected.len();
            result
        } else {
            let err = Self::error_literal(self.offset);
            Err(json_parse_error_to_serde(self.remaining_input(), err))
        }
    }

    fn consume_number<V>(&mut self, visitor: V) -> Result<V::Value, crate::serde_error::Error>
    where
        V: Visitor<'de>,
    {
        let remaining = self.remaining_input();
        let mut p = Parser::new(remaining);
        let num = p
            .parse_number()
            .map_err(|e| json_parse_error_to_serde(remaining, e))?;
        self.offset += p.index();
        match num {
            JsonNumber::I64(v) => visitor.visit_i64(v),
            JsonNumber::U64(v) => visitor.visit_u64(v),
            JsonNumber::F64(v) => visitor.visit_f64(v),
        }
    }

    fn consume_string<V>(&mut self, visitor: V) -> Result<V::Value, crate::serde_error::Error>
    where
        V: Visitor<'de>,
    {
        let remaining = self.remaining_input();
        let mut p = Parser::new(remaining);
        let s = p
            .parse_string()
            .map_err(|e| json_parse_error_to_serde(remaining, e))?;
        self.offset += p.index();
        visitor.visit_string(s)
    }

    fn consume_array<V>(&mut self, visitor: V) -> Result<V::Value, crate::serde_error::Error>
    where
        V: Visitor<'de>,
    {
        self.offset += 1; // consume '['
        let mut seq = DirectSeqAccess { de: self };
        visitor.visit_seq(&mut seq)
    }

    fn consume_object<V>(&mut self, visitor: V) -> Result<V::Value, crate::serde_error::Error>
    where
        V: Visitor<'de>,
    {
        self.offset += 1; // consume '{'
        let mut map = DirectMapAccess { de: self };
        visitor.visit_map(&mut map)
    }
}

impl<'de> SerdeDeserializer<'de> for &mut Deserializer<'de> {
    type Error = crate::serde_error::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_direct(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.peek_byte() == Some(b'n') {
            self.consume_literal(b"null", visitor.visit_none())
        } else {
            visitor.visit_some(self)
        }
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
        match self.peek_byte() {
            Some(b'{') => {
                self.offset += 1; // consume '{'
                self.skip_whitespace();
                if self.peek_byte() != Some(b'"') {
                    let err = crate::JsonParseError::UnexpectedEnd;
                    return Err(json_parse_error_to_serde(self.remaining_input(), err));
                }
                let variant_name = self.read_string()?;
                self.skip_whitespace();
                if self.remaining_input().as_bytes().starts_with(b":") {
                    self.offset += 1;
                }
                visitor.visit_enum(DirectEnumAccess {
                    variant: variant_name,
                    de: self,
                })
            }
            Some(b'"') => {
                let variant = self.read_string()?;
                visitor.visit_enum(UnitEnumAccess { variant })
            }
            _ => {
                let err = crate::JsonParseError::UnexpectedEnd;
                Err(json_parse_error_to_serde(self.remaining_input(), err))
            }
        }
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
            self.skip_whitespace();
            let start = self.offset;
            let remaining = self.remaining_input();
            let mut p = Parser::new(remaining);
            p.parse_value()
                .map_err(|e| json_parse_error_to_serde(remaining, e))?;
            let end = start + p.index();
            let raw_json = &self.input[start..end];
            self.offset = end;
            return visitor.visit_map(crate::raw::OwnedRawDeserializer::new(raw_json.to_owned()));
        }
        visitor.visit_newtype_struct(self)
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
        self.consume_object(visitor)
    }

    serde_crate::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map identifier ignored_any
    }
}

impl<'de> Deserializer<'de> {
    /// Parse a JSON string and advance offset. Used by enum variant parsing.
    fn read_string(&mut self) -> Result<String, crate::serde_error::Error> {
        let remaining = self.remaining_input();
        let mut p = Parser::new(remaining);
        let s = p
            .parse_string()
            .map_err(|e| json_parse_error_to_serde(remaining, e))?;
        self.offset += p.index();
        Ok(s)
    }
}

// ---------------------------------------------------------------------------
// DirectSeqAccess — single-pass array deserialization
// ---------------------------------------------------------------------------

struct DirectSeqAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'de> SeqAccess<'de> for DirectSeqAccess<'_, 'de> {
    type Error = crate::serde_error::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.de.peek_byte() {
            Some(b']') => {
                self.de.offset += 1;
                Ok(None)
            }
            Some(b',') => {
                self.de.offset += 1;
                Some(seed.deserialize(&mut *self.de)).transpose()
            }
            Some(_) => Some(seed.deserialize(&mut *self.de)).transpose(),
            None => {
                let err = crate::JsonParseError::UnexpectedEnd;
                Err(json_parse_error_to_serde(self.de.remaining_input(), err))
            }
        }
    }

    fn size_hint(&self) -> Option<usize> {
        None
    }
}

// ---------------------------------------------------------------------------
// DirectMapAccess — single-pass object deserialization
// ---------------------------------------------------------------------------

struct DirectMapAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'de> MapAccess<'de> for DirectMapAccess<'_, 'de> {
    type Error = crate::serde_error::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.de.peek_byte() {
            Some(b'}') => {
                self.de.offset += 1;
                Ok(None)
            }
            Some(b',') => {
                self.de.offset += 1;
                self.de.skip_whitespace();
                let key = self.de.read_string()?;
                self.de.skip_whitespace();
                if self.de.remaining_input().as_bytes().starts_with(b":") {
                    self.de.offset += 1;
                }
                seed.deserialize(StringDeserializer::new(key)).map(Some)
            }
            Some(_) => {
                let key = self.de.read_string()?;
                self.de.skip_whitespace();
                if self.de.remaining_input().as_bytes().starts_with(b":") {
                    self.de.offset += 1;
                }
                seed.deserialize(StringDeserializer::new(key)).map(Some)
            }
            None => {
                let err = crate::JsonParseError::UnexpectedEnd;
                Err(json_parse_error_to_serde(self.de.remaining_input(), err))
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        None
    }
}

// ---------------------------------------------------------------------------
// Enum access helpers
// ---------------------------------------------------------------------------

struct DirectEnumAccess<'a, 'de: 'a> {
    variant: String,
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de: 'a> EnumAccess<'de> for DirectEnumAccess<'a, 'de> {
    type Error = crate::serde_error::Error;
    type Variant = &'a mut Deserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(StringDeserializer::<Self::Error>::new(self.variant))?;
        Ok((variant, self.de))
    }
}

impl<'a, 'de: 'a> VariantAccess<'de> for &'a mut Deserializer<'de> {
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

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_array(visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_object(visitor)
    }
}

struct UnitEnumAccess {
    variant: String,
}

impl<'de> EnumAccess<'de> for UnitEnumAccess {
    type Error = crate::serde_error::Error;
    type Variant = Impossible<(), crate::serde_error::Error>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(StringDeserializer::<Self::Error>::new(self.variant))?;
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

// ---------------------------------------------------------------------------
// JsonValueDeserializer — for from_value (deserializes from existing JsonValue)
// ---------------------------------------------------------------------------

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
                let len = obj.len();
                let entries: Vec<(String, JsonValue)> =
                    obj.0.into_iter().map(|(k, v)| (k, v)).collect();
                let mut map = JsonMapAccess {
                    iter: entries.into_iter(),
                    len,
                    pending_value: None,
                };
                visitor.visit_map(&mut map)
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

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
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

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
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
            JsonValue::String(s) => visitor.visit_string(s),
            other => Err(Self::invalid_type("bytes", &other)),
        }
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

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            JsonValue::Object(obj) => {
                let len = obj.len();
                let entries: Vec<(String, JsonValue)> =
                    obj.0.into_iter().map(|(k, v)| (k, v)).collect();
                let mut map = JsonMapAccess {
                    iter: entries.into_iter(),
                    len,
                    pending_value: None,
                };
                visitor.visit_map(&mut map)
            }
            other => Err(Self::invalid_type("map", &other)),
        }
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
                let mut iter = obj.0.into_iter();
                if let Some((key, value)) = iter.next() {
                    let seed = EnumVariantAccess2 {
                        variant: key,
                        value,
                    };
                    visitor.visit_enum(seed)
                } else {
                    Err(Self::invalid_type("enum", &JsonValue::Null))
                }
            }
            JsonValue::String(variant) => visitor.visit_enum(UnitEnumAccess { variant }),
            other => Err(Self::invalid_type("enum", &other)),
        }
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

    serde_crate::forward_to_deserialize_any! {
        i8 i16 i32 i128 u8 u16 u32 u128 f32 char
        byte_buf unit_struct tuple tuple_struct struct identifier ignored_any
    }
}

struct EnumVariantAccess2 {
    variant: String,
    value: JsonValue,
}

impl<'de> EnumAccess<'de> for EnumVariantAccess2 {
    type Error = crate::serde_error::Error;
    type Variant = JsonValueDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(StringDeserializer::<Self::Error>::new(self.variant))?;
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

// ---------------------------------------------------------------------------
// serde Deserialize impls for JsonValue, JsonNumber, Map
// ---------------------------------------------------------------------------

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
        Ok(JsonValue::Number(
            JsonNumber::from_f64(value).unwrap_or(JsonNumber::F64(value)),
        ))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
        Ok(JsonValue::String(value.to_owned()))
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
            obj.insert(key, value);
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
        JsonNumber::from_f64(value).ok_or_else(|| E::custom("invalid floating point number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde_crate::de::Error,
    {
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
            obj.insert(key, value);
        }
        Ok(obj)
    }
}

// ---------------------------------------------------------------------------
// Helpers for JsonValueDeserializer (tree-based, for from_value)
// ---------------------------------------------------------------------------

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
                seed.deserialize(StringDeserializer::<Self::Error>::new(key))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self
            .pending_value
            .take()
            .expect("next_value_seed called before next_key_seed");
        seed.deserialize(JsonValueDeserializer::new(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}
