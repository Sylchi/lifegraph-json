#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::map::Map;
use crate::number::JsonNumber;
use crate::parse::{parse_i64_fast, parse_u64_fast, Parser};
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

    #[inline]
    fn peek_byte(&mut self) -> Option<u8> {
        self.skip_whitespace();
        self.remaining_input().as_bytes().first().copied()
    }

    #[inline]
    fn peek_byte_nws(&mut self) -> Option<u8> {
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
        let input = self.input.as_bytes();
        let start = self.offset;

        let negative = input.get(self.offset) == Some(&b'-');
        if negative {
            self.offset += 1;
        }

        let mut uint_val: u64 = 0;
        let mut uint_overflow = false;

        if let Some(&b'0') = input.get(self.offset) {
            self.offset += 1;
            if let Some(&digit) = input.get(self.offset) {
                if digit.is_ascii_digit() {
                    return Err(json_parse_error_to_serde(
                        &self.input[start..],
                        Self::error_literal(start),
                    ));
                }
            }
        } else {
            let digit_start = self.offset;
            while let Some(&b) = input.get(self.offset) {
                if !b.is_ascii_digit() {
                    break;
                }
                let digit = u64::from(b - b'0');
                let (new_val, overflow) = uint_val.overflowing_mul(10);
                let (new_val, overflow2) = new_val.overflowing_add(digit);
                if overflow || overflow2 {
                    uint_overflow = true;
                }
                uint_val = new_val;
                self.offset += 1;
            }
            if self.offset == digit_start {
                return Err(json_parse_error_to_serde(
                    &self.input[start..],
                    Self::error_literal(start),
                ));
            }
        }

        let mut is_float = false;
        if input.get(self.offset) == Some(&b'.') {
            is_float = true;
            self.offset += 1;
            let frac_start = self.offset;
            while let Some(&b) = input.get(self.offset) {
                if !b.is_ascii_digit() {
                    break;
                }
                self.offset += 1;
            }
            if self.offset == frac_start {
                return Err(json_parse_error_to_serde(
                    &self.input[start..],
                    Self::error_literal(start),
                ));
            }
        }
        if matches!(input.get(self.offset), Some(b'e' | b'E')) {
            is_float = true;
            self.offset += 1;
            if matches!(input.get(self.offset), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            let exp_start = self.offset;
            while let Some(&b) = input.get(self.offset) {
                if !b.is_ascii_digit() {
                    break;
                }
                self.offset += 1;
            }
            if self.offset == exp_start {
                return Err(json_parse_error_to_serde(
                    &self.input[start..],
                    Self::error_literal(start),
                ));
            }
        }

        if is_float {
            let token = &self.input[start..self.offset];
            let value = token
                .parse::<f64>()
                .map_err(|_| json_parse_error_to_serde(token, Self::error_literal(start)))?;
            if !value.is_finite() {
                return Err(json_parse_error_to_serde(token, Self::error_literal(start)));
            }
            visitor.visit_f64(value)
        } else if negative {
            if uint_overflow || uint_val > (i64::MAX as u64) + 1 {
                let token = &self.input[start..self.offset];
                let value = token
                    .parse::<i64>()
                    .map_err(|_| json_parse_error_to_serde(token, Self::error_literal(start)))?;
                visitor.visit_i64(value)
            } else {
                visitor.visit_i64(uint_val.wrapping_neg() as i64)
            }
        } else {
            if uint_overflow {
                let token = &self.input[start..self.offset];
                let value = token
                    .parse::<f64>()
                    .map_err(|_| json_parse_error_to_serde(token, Self::error_literal(start)))?;
                visitor.visit_f64(value)
            } else {
                visitor.visit_u64(uint_val)
            }
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

    #[allow(unused_variables)]
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

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_array(visitor)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_array(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_object(visitor)
    }

    // Typed numeric deserializers — avoid the full type-detection path.
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_integer(visitor, |token, visitor| {
            if let Some(i) = parse_i64_fast(token) {
                visitor.visit_i64(i)
            } else {
                // Fallback for very large numbers
                visitor.visit_i64(token.parse::<f64>().unwrap_or(0.0) as i64)
            }
        })
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_integer(visitor, |token, visitor| {
            if let Some(u) = parse_u64_fast(token) {
                visitor.visit_u64(u)
            } else {
                visitor.visit_u64(token.parse::<f64>().unwrap_or(0.0) as u64)
            }
        })
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_integer(visitor, |token, visitor| {
            visitor.visit_f64(token.parse::<f64>().unwrap_or(0.0))
        })
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_string(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.failed {
            return Err(self.error.clone().unwrap_or_else(|| {
                crate::serde_error::Error::custom("deserializer is in a failed state")
            }));
        }
        match self.peek_byte() {
            Some(b't') => self.consume_literal(b"true", visitor.visit_bool(true)),
            Some(b'f') => self.consume_literal(b"false", visitor.visit_bool(false)),
            Some(found) => {
                let err = <Deserializer<'de>>::error_unexpected(self.offset, found as char);
                Err(json_parse_error_to_serde(self.remaining_input(), err))
            }
            None => {
                let err = crate::JsonParseError::UnexpectedEnd;
                Err(json_parse_error_to_serde("", err))
            }
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_string(visitor)
    }

    // These still go through the general path (they need full type info).
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_direct(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_direct(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_direct(visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_direct(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.consume_array(visitor)
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

// ---------------------------------------------------------------------------
// DirectSeqAccess — single-pass array deserialization
// ---------------------------------------------------------------------------

struct DirectSeqAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
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

    /// Parse a JSON number token and feed it to the visitor via a callback.
    /// Used by typed numeric deserializers to avoid full type detection.
    fn consume_integer<V, F>(
        &mut self,
        visitor: V,
        f: F,
    ) -> Result<V::Value, crate::serde_error::Error>
    where
        V: Visitor<'de>,
        F: FnOnce(&str, V) -> Result<V::Value, crate::serde_error::Error>,
    {
        if self.failed {
            return Err(self.error.clone().unwrap_or_else(|| {
                crate::serde_error::Error::custom("deserializer is in a failed state")
            }));
        }
        let start = self.offset;
        let bytes = &self.input.as_bytes()[start..];

        // Skip whitespace already done by caller via peek_byte
        if bytes
            .first()
            .map_or(true, |&b| !matches!(b, b'-' | b'0'..=b'9'))
        {
            let err = crate::JsonParseError::InvalidNumber { index: start };
            return Err(json_parse_error_to_serde(&self.input[start..], err));
        }

        let mut i = 0usize;
        if bytes[i] == b'-' {
            i += 1;
        }

        // Integer digits
        if i < bytes.len() && bytes[i] == b'0' {
            i += 1;
        } else {
            let digit_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == digit_start {
                let err = crate::JsonParseError::InvalidNumber { index: start };
                return Err(json_parse_error_to_serde(&self.input[start..], err));
            }
        }

        // Skip fractional and exponent parts (they make this a float)
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
            i += 1;
            if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
                i += 1;
            }
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Safety: start + i <= input_len because i < bytes.len() and bytes = input[start..]
        let token_str = &self.input[start..start + i];
        self.offset = start + i;
        f(token_str, visitor)
    }
}

// ---------------------------------------------------------------------------
// DirectSeqAccess — single-pass array deserialization
// ---------------------------------------------------------------------------

impl<'de> SeqAccess<'de> for DirectSeqAccess<'_, 'de> {
    type Error = crate::serde_error::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.skip_whitespace();
        match self.de.peek_byte_nws() {
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
        // Skip whitespace once
        self.de.skip_whitespace();
        match self.de.peek_byte_nws() {
            Some(b'}') => {
                self.de.offset += 1;
                Ok(None)
            }
            Some(b',') => {
                self.de.offset += 1; // consume ','
                self.de.skip_whitespace(); // skip ws after comma
                let key = self.de.read_string()?;
                self.de.skip_whitespace();
                if self.de.remaining_input().as_bytes().starts_with(b":") {
                    self.de.offset += 1;
                }
                seed.deserialize(StringDeserializer::new(key)).map(Some)
            }
            Some(b'"') => {
                let key = self.de.read_string()?;
                self.de.skip_whitespace();
                if self.de.remaining_input().as_bytes().starts_with(b":") {
                    self.de.offset += 1;
                }
                seed.deserialize(StringDeserializer::new(key)).map(Some)
            }
            Some(found) => {
                let err = <Deserializer<'de>>::error_unexpected(self.de.offset, found as char);
                Err(json_parse_error_to_serde(self.de.remaining_input(), err))
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
                let entries: Vec<(String, JsonValue)> = obj.0.into_iter().collect();
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
                let entries: Vec<(String, JsonValue)> = obj.0.into_iter().collect();
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
        if let Some(n) = parse_i64_fast(value) {
            return Ok(JsonNumber::from(n));
        }
        if let Some(n) = parse_u64_fast(value) {
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
