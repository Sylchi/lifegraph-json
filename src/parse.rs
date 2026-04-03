use crate::{JsonParseError, JsonValue};
use crate::tape::{BorrowedJsonValue, JsonTape, TapeToken, TapeTokenKind};
use std::borrow::Cow;

pub struct Parser<'a> {
    pub input: &'a str,
    bytes: &'a [u8],
    pub index: usize,
    pub offset: usize,
    pub failed: bool,
    pub error: Option<JsonParseError>,
    pub depth: usize,
    max_depth: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            index: 0,
            offset: 0,
            failed: false,
            error: None,
            depth: 0,
            max_depth: 10000, // Match serde_json's default
        }
    }

    pub fn from_string(input: String) -> Self {
        let input: &'static str = Box::leak(input.into_boxed_str());
        Self {
            input,
            bytes: input.as_bytes(),
            index: 0,
            offset: 0,
            failed: false,
            error: None,
            depth: 0,
            max_depth: 10000,
        }
    }

    #[cfg(feature = "std")]
    pub fn from_reader<R: std::io::Read>(mut reader: R) -> Self {
        let mut input = String::new();
        match reader.read_to_string(&mut input) {
            Ok(_) => Self::from_string(input),
            Err(_) => Self {
                input: "",
                bytes: b"",
                index: 0,
                offset: 0,
                failed: true,
                error: Some(crate::error::JsonParseError::InvalidUtf8),
                depth: 0,
                max_depth: 10000,
            },
        }
    }

    pub fn parse_value(&mut self) -> Result<JsonValue, JsonParseError> {
        self.skip_whitespace();
        // Check depth before parsing nested structures
        match self.peek_byte() {
            Some(b'[' | b'{') => {
                if self.depth >= self.max_depth {
                    return Err(JsonParseError::NestingTooDeep {
                        depth: self.depth,
                        max: self.max_depth,
                    });
                }
                self.depth += 1;
            }
            _ => {}
        }
        
        match self.peek_byte() {
            Some(b'n') => self.parse_literal(b"null", JsonValue::Null),
            Some(b't') => self.parse_literal(b"true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal(b"false", JsonValue::Bool(false)),
            Some(b'"') => Ok(JsonValue::String(self.parse_string()?)),
            Some(b'[') => {
                let result = self.parse_array();
                self.depth -= 1;
                result
            }
            Some(b'{') => {
                let result = self.parse_object();
                self.depth -= 1;
                result
            }
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(found) => Err(JsonParseError::UnexpectedCharacter {
                index: self.index,
                found: found as char,
            }),
            None => Err(JsonParseError::UnexpectedEnd),
        }
    }

    pub fn parse_value_borrowed(&mut self) -> Result<BorrowedJsonValue<'a>, JsonParseError> {
        self.skip_whitespace();
        // Check depth before parsing nested structures
        match self.peek_byte() {
            Some(b'[' | b'{') => {
                if self.depth >= self.max_depth {
                    return Err(JsonParseError::NestingTooDeep {
                        depth: self.depth,
                        max: self.max_depth,
                    });
                }
                self.depth += 1;
            }
            _ => {}
        }
        
        match self.peek_byte() {
            Some(b'n') => self.parse_literal_borrowed(b"null", BorrowedJsonValue::Null),
            Some(b't') => self.parse_literal_borrowed(b"true", BorrowedJsonValue::Bool(true)),
            Some(b'f') => self.parse_literal_borrowed(b"false", BorrowedJsonValue::Bool(false)),
            Some(b'"') => Ok(BorrowedJsonValue::String(self.parse_string_borrowed()?)),
            Some(b'[') => {
                let result = self.parse_array_borrowed();
                self.depth -= 1;
                result
            }
            Some(b'{') => {
                let result = self.parse_object_borrowed();
                self.depth -= 1;
                result
            }
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(BorrowedJsonValue::Number),
            Some(found) => Err(JsonParseError::UnexpectedCharacter {
                index: self.index,
                found: found as char,
            }),
            None => Err(JsonParseError::UnexpectedEnd),
        }
    }

    pub fn parse_tape_value(
        &mut self,
        tokens: &mut Vec<TapeToken>,
        parent: Option<usize>,
    ) -> Result<usize, JsonParseError> {
        self.skip_whitespace();
        match self.peek_byte() {
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
        }
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
        // Check depth before parsing
        if self.depth >= self.max_depth {
            return Err(JsonParseError::NestingTooDeep {
                depth: self.depth,
                max: self.max_depth,
            });
        }
        self.depth += 1;
        
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
            self.depth -= 1;
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
        self.depth -= 1;
        Ok(token_index)
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonParseError> {
        self.consume_byte(b'{')?;
        self.skip_whitespace();
        let mut entries = crate::Map::new();
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
        // Check depth before parsing
        if self.depth >= self.max_depth {
            return Err(JsonParseError::NestingTooDeep {
                depth: self.depth,
                max: self.max_depth,
            });
        }
        self.depth += 1;
        
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
            self.depth -= 1;
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
        self.depth -= 1;
        Ok(token_index)
    }

    fn parse_string(&mut self) -> Result<String, JsonParseError> {
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
                                        index: self.index,
                                    });
                                }
                                let low = self.parse_hex_quad(escape_index)?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(JsonParseError::InvalidUnicodeScalar {
                                        index: self.index,
                                    });
                                }
                            } else if (0xDC00..=0xDFFF).contains(&scalar) {
                                return Err(JsonParseError::InvalidUnicodeScalar {
                                    index: self.index,
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

    fn parse_escape_into(&mut self, out: &mut String, start: usize) -> Result<(), JsonParseError> {
        let Some(byte) = self.next_byte() else {
            return Err(JsonParseError::UnexpectedEnd);
        };
        match byte {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\x08'),
            b'f' => out.push('\x0c'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => {
                let scalar = self.parse_hex_quad(start)?;
                out.push(char::from_u32(scalar).ok_or(JsonParseError::InvalidUnicodeScalar {
                    index: self.index,
                })?);
            }
            _ => {
                return Err(JsonParseError::InvalidEscape { index: start });
            }
        }
        Ok(())
    }

    fn parse_hex_quad(&mut self, start: usize) -> Result<u32, JsonParseError> {
        if self.index + 4 > self.bytes.len() {
            return Err(JsonParseError::InvalidUnicodeEscape { index: start });
        }
        let mut value = 0u32;
        for _ in 0..4 {
            let byte = self.next_byte().unwrap();
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'A'..=b'F' => byte - b'A' + 10,
                b'a'..=b'f' => byte - b'a' + 10,
                _ => {
                    return Err(JsonParseError::InvalidUnicodeEscape { index: start });
                }
            };
            value = (value << 4) | digit as u32;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<crate::JsonNumber, JsonParseError> {
        let start = self.index;
        let negative = self.try_consume_byte(b'-');
        let start_digits = self.index;
        self.consume_digits();
        let end = self.index;
        let end_with_dot = end;
        let has_exponent = if matches!(self.peek_byte(), Some(b'e') | Some(b'E')) {
            self.index += 1;
            if matches!(self.peek_byte(), Some(b'+') | Some(b'-')) {
                self.index += 1;
            }
            let exp_start = self.index;
            self.consume_digits();
            self.index > exp_start
        } else {
            false
        };
        let value = &self.input[start_digits..end];
        if end_with_dot == start_digits {
            return Err(JsonParseError::InvalidNumber { index: start });
        }
        if !has_exponent && end == end_with_dot {
            if negative {
                let Ok(val) = value.parse::<i64>() else {
                    return Err(JsonParseError::InvalidNumber { index: start });
                };
                Ok(crate::JsonNumber::I64(val))
            } else {
                let Ok(val) = value.parse::<u64>() else {
                    return Err(JsonParseError::InvalidNumber { index: start });
                };
                Ok(crate::JsonNumber::U64(val))
            }
        } else {
            let Ok(val) = value.parse::<f64>() else {
                return Err(JsonParseError::InvalidNumber { index: start });
            };
            Ok(crate::JsonNumber::F64(val))
        }
    }

    fn parse_tape_number(
        &mut self,
        tokens: &mut Vec<TapeToken>,
        parent: Option<usize>,
    ) -> Result<usize, JsonParseError> {
        let start = self.index;
        let _ = self.try_consume_byte(b'-');
        let start_digits = self.index;
        self.consume_digits();
        let end = self.index;
        let end_with_dot = end;
        let has_exponent = if matches!(self.peek_byte(), Some(b'e') | Some(b'E')) {
            self.index += 1;
            if matches!(self.peek_byte(), Some(b'+') | Some(b'-')) {
                self.index += 1;
            }
            let exp_start = self.index;
            self.consume_digits();
            self.index > exp_start
        } else {
            false
        };
        let token_index = tokens.len();
        tokens.push(TapeToken {
            kind: TapeTokenKind::Number,
            start,
            end: self.index,
            parent,
        });
        if end_with_dot == start_digits {
            return Err(JsonParseError::InvalidNumber { index: start });
        }
        if !has_exponent && end == end_with_dot {
            Ok(token_index)
        } else {
            let value = &self.input[start_digits..end];
            let Ok(_) = value.parse::<f64>() else {
                return Err(JsonParseError::InvalidNumber { index: start });
            };
            Ok(token_index)
        }
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
            self.index += 1;
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
        let byte = self.bytes.get(self.index).copied();
        if byte.is_some() {
            self.index += 1;
        }
        byte
    }

    fn try_consume_byte(&mut self, byte: u8) -> bool {
        if self.peek_byte() == Some(byte) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn consume_byte(&mut self, byte: u8) -> Result<(), JsonParseError> {
        if self.try_consume_byte(byte) {
            Ok(())
        } else {
            Err(JsonParseError::UnexpectedCharacter {
                index: self.index,
                found: byte as char,
            })
        }
    }

    pub fn is_eof(&self) -> bool {
        self.index >= self.bytes.len()
    }
}

pub fn parse_json(input: &str) -> Result<JsonValue, JsonParseError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(value)
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index))
    }
}

pub fn parse_json_borrowed(input: &str) -> Result<BorrowedJsonValue<'_>, JsonParseError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value_borrowed()?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(value)
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index))
    }
}

pub fn parse_json_tape(input: &str) -> Result<JsonTape, JsonParseError> {
    let mut parser = Parser::new(input);
    let mut tokens = Vec::new();
    parser.parse_tape_value(&mut tokens, None)?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(JsonTape { tokens })
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index))
    }
}

pub fn parse_string_token_borrowed(input: &str) -> Result<Cow<'_, str>, JsonParseError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_string_borrowed()?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(value)
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index))
    }
}
