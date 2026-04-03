#[cfg(not(feature = "std"))]
use alloc::borrow::ToOwned;

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::borrowed_value::BorrowedJsonValue;
use crate::error::JsonParseError;
use crate::map::Map;
use crate::number::JsonNumber;
use crate::tape::{TapeToken, TapeTokenKind};
use crate::JsonValue;
#[cfg(not(feature = "std"))]
use alloc::borrow::Cow;
#[cfg(feature = "std")]
use std::borrow::Cow;

/// Optimized unsigned 64-bit integer parser.
/// Parses a decimal string slice into u64 without allocation.
/// Returns None on overflow or invalid input.
pub(crate) fn parse_u64_fast(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }

    let bytes = s.as_bytes();

    // Fast path for small numbers (common case)
    if bytes.len() <= 3 {
        let mut val: u64 = 0;
        for &b in bytes {
            if !b.is_ascii_digit() {
                return None;
            }
            val = val * 10 + (b - b'0') as u64;
        }
        return Some(val);
    }

    // Validate all bytes are digits first
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
    }

    // Process 18 digits at a time using u128 to avoid overflow checks
    let mut result: u64;
    let mut pos: usize;

    // Handle first chunk specially to avoid initial zero
    let chunk_size = if bytes.len() % 18 == 0 {
        18
    } else {
        bytes.len() % 18
    };

    // Parse first chunk
    let mut chunk_val: u64 = 0;
    for &byte in bytes.iter().take(chunk_size) {
        chunk_val = chunk_val * 10 + (byte - b'0') as u64;
    }
    result = chunk_val;
    pos = chunk_size;

    // Process remaining chunks of 18 digits
    while pos < bytes.len() {
        let chunk_end = (pos + 18).min(bytes.len());

        // Use u128 for intermediate calculation to handle potential overflow
        let mut chunk_u128: u128 = 0;
        for &byte in bytes.iter().skip(pos).take(chunk_end - pos) {
            chunk_u128 = chunk_u128 * 10 + (byte - b'0') as u128;
        }

        // Check if chunk fits in u64
        if chunk_u128 > u64::MAX as u128 {
            return None; // Overflow
        }
        let chunk_val = chunk_u128 as u64;

        // Multiply result by 10^18 and add chunk
        // Check for overflow
        let multiplier = 10u64.pow((chunk_end - pos) as u32);
        let (new_result, overflow) = result.overflowing_mul(multiplier);
        if overflow {
            return None;
        }
        let (new_result, overflow) = new_result.overflowing_add(chunk_val);
        if overflow {
            return None;
        }
        result = new_result;

        pos = chunk_end;
    }

    Some(result)
}

/// Optimized signed 64-bit integer parser.
/// Parses a decimal string slice (possibly with leading '-') into i64.
/// Returns None on overflow or invalid input.
pub(crate) fn parse_i64_fast(s: &str) -> Option<i64> {
    if s.is_empty() {
        return None;
    }

    let bytes = s.as_bytes();
    let (negative, digits) = if bytes[0] == b'-' {
        if bytes.len() == 1 {
            return None; // Just "-" is invalid
        }
        (true, &bytes[1..])
    } else {
        (false, bytes)
    };

    // Parse as unsigned first
    let unsigned_val = parse_u64_fast(core::str::from_utf8(digits).ok()?)?;

    if negative {
        // For negative numbers, we need to handle i64::MIN specially
        // i64::MIN = -9223372036854775808, which is one more than i64::MAX in absolute value
        if unsigned_val > (i64::MAX as u64) + 1 {
            return None; // Overflow
        }
        // Handle i64::MIN specially to avoid overflow with negation
        if unsigned_val == (i64::MAX as u64) + 1 {
            Some(i64::MIN)
        } else {
            // Safe to negate because unsigned_val <= i64::MAX
            Some(-(unsigned_val as i64))
        }
    } else {
        if unsigned_val > i64::MAX as u64 {
            return None; // Overflow for i64
        }
        Some(unsigned_val as i64)
    }
}

pub(crate) struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    index: usize,
    depth: usize,
    max_depth: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            index: 0,
            depth: 0,
            max_depth: 128,
        }
    }

    pub fn parse_value(&mut self) -> Result<JsonValue, JsonParseError> {
        self.skip_whitespace();
        let is_nested = matches!(self.peek_byte(), Some(b'[' | b'{'));
        if is_nested {
            if self.depth >= self.max_depth {
                return Err(JsonParseError::NestingTooDeep {
                    depth: self.depth,
                    max: self.max_depth,
                });
            }
            self.depth += 1;
        }

        let result = match self.peek_byte() {
            Some(b'n') => self.parse_literal(b"null", JsonValue::Null),
            Some(b't') => self.parse_literal(b"true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal(b"false", JsonValue::Bool(false)),
            Some(b'"') => Ok(JsonValue::String(self.parse_string()?)),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(found) => Err(JsonParseError::UnexpectedCharacter {
                index: self.index,
                found: found as char,
            }),
            None => Err(JsonParseError::UnexpectedEnd),
        };

        if is_nested {
            self.depth -= 1;
        }

        result
    }

    pub fn parse_value_borrowed(&mut self) -> Result<BorrowedJsonValue<'a>, JsonParseError> {
        self.skip_whitespace();
        let is_nested = matches!(self.peek_byte(), Some(b'[' | b'{'));
        if is_nested {
            if self.depth >= self.max_depth {
                return Err(JsonParseError::NestingTooDeep {
                    depth: self.depth,
                    max: self.max_depth,
                });
            }
            self.depth += 1;
        }

        let result = match self.peek_byte() {
            Some(b'n') => self.parse_literal_borrowed(b"null", BorrowedJsonValue::Null),
            Some(b't') => self.parse_literal_borrowed(b"true", BorrowedJsonValue::Bool(true)),
            Some(b'f') => self.parse_literal_borrowed(b"false", BorrowedJsonValue::Bool(false)),
            Some(b'"') => Ok(BorrowedJsonValue::String(self.parse_string_borrowed()?)),
            Some(b'[') => self.parse_array_borrowed(),
            Some(b'{') => self.parse_object_borrowed(),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(BorrowedJsonValue::Number),
            Some(found) => Err(JsonParseError::UnexpectedCharacter {
                index: self.index,
                found: found as char,
            }),
            None => Err(JsonParseError::UnexpectedEnd),
        };

        if is_nested {
            self.depth -= 1;
        }

        result
    }

    pub fn parse_tape_value(
        &mut self,
        tokens: &mut Vec<TapeToken>,
        parent: Option<usize>,
    ) -> Result<usize, JsonParseError> {
        self.skip_whitespace();
        let is_nested = matches!(self.peek_byte(), Some(b'[' | b'{'));
        if is_nested {
            if self.depth >= self.max_depth {
                return Err(JsonParseError::NestingTooDeep {
                    depth: self.depth,
                    max: self.max_depth,
                });
            }
            self.depth += 1;
        }

        let result = match self.peek_byte() {
            Some(b'n') => self.parse_tape_literal(tokens, parent, b"null", TapeTokenKind::Null),
            Some(b't') => self.parse_tape_literal(tokens, parent, b"true", TapeTokenKind::Bool),
            Some(b'f') => self.parse_tape_literal(tokens, parent, b"false", TapeTokenKind::Bool),
            Some(b'"') => self.parse_tape_string(tokens, parent, TapeTokenKind::String),
            Some(b'[') => self.parse_tape_array(tokens, parent),
            Some(b'{') => self.parse_tape_object(tokens, parent),
            Some(b'-' | b'0'..=b'9') => self.parse_tape_number(tokens, parent),
            Some(found) => Err(JsonParseError::UnexpectedCharacter {
                index: self.index,
                found: found as char,
            }),
            None => Err(JsonParseError::UnexpectedEnd),
        };

        if is_nested {
            self.depth -= 1;
        }

        result
    }

    fn parse_literal(
        &mut self,
        expected: &[u8],
        value: JsonValue,
    ) -> Result<JsonValue, JsonParseError> {
        if self.bytes[self.index..].starts_with(expected) {
            self.index += expected.len();
            Ok(value)
        } else {
            Err(JsonParseError::InvalidLiteral { index: self.index })
        }
    }

    fn parse_literal_borrowed(
        &mut self,
        expected: &[u8],
        value: BorrowedJsonValue<'a>,
    ) -> Result<BorrowedJsonValue<'a>, JsonParseError> {
        if self.bytes[self.index..].starts_with(expected) {
            self.index += expected.len();
            Ok(value)
        } else {
            Err(JsonParseError::InvalidLiteral { index: self.index })
        }
    }

    fn parse_tape_literal(
        &mut self,
        tokens: &mut Vec<TapeToken>,
        parent: Option<usize>,
        expected: &[u8],
        kind: TapeTokenKind,
    ) -> Result<usize, JsonParseError> {
        let start = self.index;
        if self.bytes[self.index..].starts_with(expected) {
            self.index += expected.len();
            let token_index = tokens.len();
            tokens.push(TapeToken {
                kind,
                start,
                end: self.index,
                parent,
            });
            Ok(token_index)
        } else {
            Err(JsonParseError::InvalidLiteral { index: self.index })
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonParseError> {
        self.consume_byte(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.try_consume_byte(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.try_consume_byte(b']') {
                break;
            }
            if !self.try_consume_byte(b',') {
                return Err(JsonParseError::ExpectedCommaOrEnd {
                    index: self.index,
                    context: "array",
                });
            }
            self.skip_whitespace();
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_array_borrowed(&mut self) -> Result<BorrowedJsonValue<'a>, JsonParseError> {
        self.consume_byte(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.try_consume_byte(b']') {
            return Ok(BorrowedJsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value_borrowed()?);
            self.skip_whitespace();
            if self.try_consume_byte(b']') {
                break;
            }
            if !self.try_consume_byte(b',') {
                return Err(JsonParseError::ExpectedCommaOrEnd {
                    index: self.index,
                    context: "array",
                });
            }
            self.skip_whitespace();
        }
        Ok(BorrowedJsonValue::Array(values))
    }

    fn parse_tape_array(
        &mut self,
        tokens: &mut Vec<TapeToken>,
        parent: Option<usize>,
    ) -> Result<usize, JsonParseError> {
        let start = self.index;
        self.consume_byte(b'[')?;
        let token_index = tokens.len();
        tokens.push(TapeToken {
            kind: TapeTokenKind::Array,
            start,
            end: start,
            parent,
        });
        self.skip_whitespace();
        if self.try_consume_byte(b']') {
            tokens[token_index].end = self.index;
            return Ok(token_index);
        }
        loop {
            self.parse_tape_value(tokens, Some(token_index))?;
            self.skip_whitespace();
            if self.try_consume_byte(b']') {
                tokens[token_index].end = self.index;
                break;
            }
            if !self.try_consume_byte(b',') {
                return Err(JsonParseError::ExpectedCommaOrEnd {
                    index: self.index,
                    context: "array",
                });
            }
            self.skip_whitespace();
        }
        tokens[token_index].end = self.index;
        Ok(token_index)
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonParseError> {
        self.consume_byte(b'{')?;
        self.skip_whitespace();
        let mut entries = Map::new();
        if self.try_consume_byte(b'}') {
            return Ok(JsonValue::Object(entries));
        }
        loop {
            if self.peek_byte() != Some(b'"') {
                return match self.peek_byte() {
                    Some(found) => Err(JsonParseError::UnexpectedCharacter {
                        index: self.index,
                        found: found as char,
                    }),
                    None => Err(JsonParseError::UnexpectedEnd),
                };
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if !self.try_consume_byte(b':') {
                return Err(JsonParseError::ExpectedColon { index: self.index });
            }
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_whitespace();
            if self.try_consume_byte(b'}') {
                break;
            }
            if !self.try_consume_byte(b',') {
                return Err(JsonParseError::ExpectedCommaOrEnd {
                    index: self.index,
                    context: "object",
                });
            }
            self.skip_whitespace();
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_object_borrowed(&mut self) -> Result<BorrowedJsonValue<'a>, JsonParseError> {
        self.consume_byte(b'{')?;
        self.skip_whitespace();
        let mut entries = Vec::new();
        if self.try_consume_byte(b'}') {
            return Ok(BorrowedJsonValue::Object(entries));
        }
        loop {
            if self.peek_byte() != Some(b'"') {
                return match self.peek_byte() {
                    Some(found) => Err(JsonParseError::UnexpectedCharacter {
                        index: self.index,
                        found: found as char,
                    }),
                    None => Err(JsonParseError::UnexpectedEnd),
                };
            }
            let key = self.parse_string_borrowed()?;
            self.skip_whitespace();
            if !self.try_consume_byte(b':') {
                return Err(JsonParseError::ExpectedColon { index: self.index });
            }
            let value = self.parse_value_borrowed()?;
            entries.push((key, value));
            self.skip_whitespace();
            if self.try_consume_byte(b'}') {
                break;
            }
            if !self.try_consume_byte(b',') {
                return Err(JsonParseError::ExpectedCommaOrEnd {
                    index: self.index,
                    context: "object",
                });
            }
            self.skip_whitespace();
        }
        Ok(BorrowedJsonValue::Object(entries))
    }

    fn parse_tape_object(
        &mut self,
        tokens: &mut Vec<TapeToken>,
        parent: Option<usize>,
    ) -> Result<usize, JsonParseError> {
        let start = self.index;
        self.consume_byte(b'{')?;
        let token_index = tokens.len();
        tokens.push(TapeToken {
            kind: TapeTokenKind::Object,
            start,
            end: start,
            parent,
        });
        self.skip_whitespace();
        if self.try_consume_byte(b'}') {
            tokens[token_index].end = self.index;
            return Ok(token_index);
        }
        loop {
            if self.peek_byte() != Some(b'"') {
                return match self.peek_byte() {
                    Some(found) => Err(JsonParseError::UnexpectedCharacter {
                        index: self.index,
                        found: found as char,
                    }),
                    None => Err(JsonParseError::UnexpectedEnd),
                };
            }
            self.parse_tape_string(tokens, Some(token_index), TapeTokenKind::Key)?;
            self.skip_whitespace();
            if !self.try_consume_byte(b':') {
                return Err(JsonParseError::ExpectedColon { index: self.index });
            }
            self.parse_tape_value(tokens, Some(token_index))?;
            self.skip_whitespace();
            if self.try_consume_byte(b'}') {
                tokens[token_index].end = self.index;
                break;
            }
            if !self.try_consume_byte(b',') {
                return Err(JsonParseError::ExpectedCommaOrEnd {
                    index: self.index,
                    context: "object",
                });
            }
            self.skip_whitespace();
        }
        tokens[token_index].end = self.index;
        Ok(token_index)
    }

    pub(crate) fn parse_string(&mut self) -> Result<String, JsonParseError> {
        self.consume_byte(b'"')?;
        let start = self.index;
        loop {
            let Some(byte) = self.next_byte() else {
                return Err(JsonParseError::UnexpectedEnd);
            };
            match byte {
                b'"' => {
                    let slice = &self.input[start..self.index - 1];
                    return Ok(slice.to_owned());
                }
                b'\\' => {
                    let mut out = String::with_capacity(self.index - start + 8);
                    out.push_str(&self.input[start..self.index - 1]);
                    self.parse_escape_into(&mut out, self.index - 1)?;
                    return self.parse_string_slow(out);
                }
                0x00..=0x1f => {
                    return Err(JsonParseError::UnexpectedCharacter {
                        index: self.index - 1,
                        found: byte as char,
                    })
                }
                _ => {}
            }
        }
    }

    fn parse_string_borrowed(&mut self) -> Result<Cow<'a, str>, JsonParseError> {
        self.consume_byte(b'"')?;
        let start = self.index;
        loop {
            let Some(byte) = self.next_byte() else {
                return Err(JsonParseError::UnexpectedEnd);
            };
            match byte {
                b'"' => {
                    let slice = &self.input[start..self.index - 1];
                    return Ok(Cow::Borrowed(slice));
                }
                b'\\' => {
                    let mut out = String::with_capacity(self.index - start + 8);
                    out.push_str(&self.input[start..self.index - 1]);
                    self.parse_escape_into(&mut out, self.index - 1)?;
                    return self.parse_string_slow_borrowed(out);
                }
                0x00..=0x1f => {
                    return Err(JsonParseError::UnexpectedCharacter {
                        index: self.index - 1,
                        found: byte as char,
                    })
                }
                _ => {}
            }
        }
    }

    fn parse_tape_string(
        &mut self,
        tokens: &mut Vec<TapeToken>,
        parent: Option<usize>,
        kind: TapeTokenKind,
    ) -> Result<usize, JsonParseError> {
        let start = self.index;
        self.skip_string_bytes()?;
        let token_index = tokens.len();
        tokens.push(TapeToken {
            kind,
            start,
            end: self.index,
            parent,
        });
        Ok(token_index)
    }

    fn skip_string_bytes(&mut self) -> Result<(), JsonParseError> {
        self.consume_byte(b'"')?;
        loop {
            let Some(byte) = self.next_byte() else {
                return Err(JsonParseError::UnexpectedEnd);
            };
            match byte {
                b'"' => return Ok(()),
                b'\\' => {
                    let escape_index = self.index - 1;
                    let escaped = self.next_byte().ok_or(JsonParseError::UnexpectedEnd)?;
                    match escaped {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                        b'u' => {
                            let scalar = self.parse_hex_quad(escape_index)?;
                            if (0xD800..=0xDBFF).contains(&scalar) {
                                if self.next_byte() != Some(b'\\') || self.next_byte() != Some(b'u')
                                {
                                    return Err(JsonParseError::InvalidUnicodeScalar {
                                        index: escape_index,
                                    });
                                }
                                let low = self.parse_hex_quad(escape_index)?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(JsonParseError::InvalidUnicodeScalar {
                                        index: escape_index,
                                    });
                                }
                            } else if (0xDC00..=0xDFFF).contains(&scalar) {
                                return Err(JsonParseError::InvalidUnicodeScalar {
                                    index: escape_index,
                                });
                            }
                        }
                        _ => {
                            return Err(JsonParseError::InvalidEscape {
                                index: escape_index,
                            })
                        }
                    }
                }
                0x00..=0x1f => {
                    return Err(JsonParseError::UnexpectedCharacter {
                        index: self.index - 1,
                        found: byte as char,
                    })
                }
                _ => {}
            }
        }
    }

    fn parse_string_slow(&mut self, mut out: String) -> Result<String, JsonParseError> {
        let mut chunk_start = self.index;
        loop {
            let Some(byte) = self.next_byte() else {
                return Err(JsonParseError::UnexpectedEnd);
            };
            match byte {
                b'"' => {
                    if chunk_start < self.index - 1 {
                        out.push_str(&self.input[chunk_start..self.index - 1]);
                    }
                    return Ok(out);
                }
                b'\\' => {
                    if chunk_start < self.index - 1 {
                        out.push_str(&self.input[chunk_start..self.index - 1]);
                    }
                    self.parse_escape_into(&mut out, self.index - 1)?;
                    chunk_start = self.index;
                }
                0x00..=0x1f => {
                    return Err(JsonParseError::UnexpectedCharacter {
                        index: self.index - 1,
                        found: byte as char,
                    })
                }
                _ => {}
            }
        }
    }

    fn parse_string_slow_borrowed(
        &mut self,
        mut out: String,
    ) -> Result<Cow<'a, str>, JsonParseError> {
        let mut chunk_start = self.index;
        loop {
            let Some(byte) = self.next_byte() else {
                return Err(JsonParseError::UnexpectedEnd);
            };
            match byte {
                b'"' => {
                    if chunk_start < self.index - 1 {
                        out.push_str(&self.input[chunk_start..self.index - 1]);
                    }
                    return Ok(Cow::Owned(out));
                }
                b'\\' => {
                    if chunk_start < self.index - 1 {
                        out.push_str(&self.input[chunk_start..self.index - 1]);
                    }
                    self.parse_escape_into(&mut out, self.index - 1)?;
                    chunk_start = self.index;
                }
                0x00..=0x1f => {
                    return Err(JsonParseError::UnexpectedCharacter {
                        index: self.index - 1,
                        found: byte as char,
                    })
                }
                _ => {}
            }
        }
    }

    fn parse_escape_into(
        &mut self,
        out: &mut String,
        escape_index: usize,
    ) -> Result<(), JsonParseError> {
        let escaped = self.next_byte().ok_or(JsonParseError::UnexpectedEnd)?;
        match escaped {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\u{0008}'),
            b'f' => out.push('\u{000C}'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => out.push(self.parse_unicode_escape(escape_index)?),
            _ => {
                return Err(JsonParseError::InvalidEscape {
                    index: escape_index,
                })
            }
        }
        Ok(())
    }

    fn parse_unicode_escape(&mut self, index: usize) -> Result<char, JsonParseError> {
        let scalar = self.parse_hex_quad(index)?;
        if (0xD800..=0xDBFF).contains(&scalar) {
            if self.next_byte() != Some(b'\\') || self.next_byte() != Some(b'u') {
                return Err(JsonParseError::InvalidUnicodeScalar { index });
            }
            let low = self.parse_hex_quad(index)?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err(JsonParseError::InvalidUnicodeScalar { index });
            }
            let high = scalar - 0xD800;
            let low = low - 0xDC00;
            let combined = 0x10000 + ((high << 10) | low);
            char::from_u32(combined).ok_or(JsonParseError::InvalidUnicodeScalar { index })
        } else if (0xDC00..=0xDFFF).contains(&scalar) {
            Err(JsonParseError::InvalidUnicodeScalar { index })
        } else {
            char::from_u32(scalar).ok_or(JsonParseError::InvalidUnicodeScalar { index })
        }
    }

    fn parse_hex_quad(&mut self, index: usize) -> Result<u32, JsonParseError> {
        let mut value = 0u32;
        for _ in 0..4 {
            let ch = self.next_byte().ok_or(JsonParseError::UnexpectedEnd)?;
            let digit = match ch {
                b'0'..=b'9' => u32::from(ch - b'0'),
                b'a'..=b'f' => 10 + u32::from(ch - b'a'),
                b'A'..=b'F' => 10 + u32::from(ch - b'A'),
                _ => return Err(JsonParseError::InvalidUnicodeEscape { index }),
            };
            value = (value << 4) | digit;
        }
        Ok(value)
    }

    pub(crate) fn parse_number(&mut self) -> Result<JsonNumber, JsonParseError> {
        let start = self.index;
        self.try_consume_byte(b'-');
        if self.try_consume_byte(b'0') {
            if matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(JsonParseError::InvalidNumber { index: start });
            }
        } else {
            self.consume_digits(start)?;
        }

        let mut is_float = false;
        if self.try_consume_byte(b'.') {
            is_float = true;
            self.consume_digits(start)?;
        }
        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            is_float = true;
            self.index += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.index += 1;
            }
            self.consume_digits(start)?;
        }

        let token = &self.input[start..self.index];
        if is_float {
            let value = token
                .parse::<f64>()
                .map_err(|_| JsonParseError::InvalidNumber { index: start })?;
            if !value.is_finite() {
                return Err(JsonParseError::InvalidNumber { index: start });
            }
            Ok(JsonNumber::F64(value))
        } else if token.starts_with('-') {
            let value =
                parse_i64_fast(token).ok_or(JsonParseError::InvalidNumber { index: start })?;
            Ok(JsonNumber::I64(value))
        } else {
            let value =
                parse_u64_fast(token).ok_or(JsonParseError::InvalidNumber { index: start })?;
            Ok(JsonNumber::U64(value))
        }
    }

    fn parse_tape_number(
        &mut self,
        tokens: &mut Vec<TapeToken>,
        parent: Option<usize>,
    ) -> Result<usize, JsonParseError> {
        let start = self.index;
        let _ = self.parse_number()?;
        let token_index = tokens.len();
        tokens.push(TapeToken {
            kind: TapeTokenKind::Number,
            start,
            end: self.index,
            parent,
        });
        Ok(token_index)
    }

    fn consume_digits(&mut self, index: usize) -> Result<(), JsonParseError> {
        let start = self.index;
        while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
            self.index += 1;
        }
        if self.index == start {
            return Err(JsonParseError::InvalidNumber { index });
        }
        Ok(())
    }

    fn consume_byte(&mut self, expected: u8) -> Result<(), JsonParseError> {
        match self.next_byte() {
            Some(found) if found == expected => Ok(()),
            Some(found) => Err(JsonParseError::UnexpectedCharacter {
                index: self.index.saturating_sub(1),
                found: found as char,
            }),
            None => Err(JsonParseError::UnexpectedEnd),
        }
    }

    fn try_consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    pub fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.index += 1;
        Some(byte)
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn is_eof(&self) -> bool {
        self.index >= self.input.len()
    }
}
