use std::borrow::Cow;
use std::fmt;
use std::io::{Read, Write};
use std::ops::{Index, IndexMut};


#[derive(Clone, Debug, PartialEq)]
pub enum JsonNumber {
    I64(i64),
    U64(u64),
    F64(f64),
}


impl JsonNumber {
    pub fn is_i64(&self) -> bool {
        matches!(self, Self::I64(_))
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

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::I64(value) => (*value >= 0).then_some(*value as u64),
            Self::U64(value) => Some(*value),
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

pub type Value = JsonValue;
pub type Number = JsonNumber;
pub type Map = Vec<(String, JsonValue)>;

#[derive(Clone, Debug, PartialEq)]
pub enum BorrowedJsonValue<'a> {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(Cow<'a, str>),
    Array(Vec<BorrowedJsonValue<'a>>),
    Object(Vec<(Cow<'a, str>, BorrowedJsonValue<'a>)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledObjectSchema {
    fields: Vec<CompiledField>,
    capacity_hint: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledRowSchema {
    object: CompiledObjectSchema,
    row_capacity_hint: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonTape {
    pub tokens: Vec<TapeToken>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapeToken {
    pub kind: TapeTokenKind,
    pub start: usize,
    pub end: usize,
    pub parent: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapeTokenKind {
    Null,
    Bool,
    Number,
    String,
    Key,
    Array,
    Object,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TapeValue<'a> {
    tape: &'a JsonTape,
    input: &'a str,
    index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapeObjectIndex {
    buckets: Vec<Vec<(u64, usize, usize)>>,
}

#[derive(Clone, Copy, Debug)]
pub struct IndexedTapeObject<'a> {
    object: TapeValue<'a>,
    index: &'a TapeObjectIndex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledTapeKey {
    key: String,
    hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledTapeKeys {
    keys: Vec<CompiledTapeKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompiledField {
    key: String,
    rendered_prefix: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonError {
    NonFiniteNumber,
    Io,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonParseError {
    InvalidUtf8,
    UnexpectedEnd,
    UnexpectedTrailingCharacters(usize),
    UnexpectedCharacter { index: usize, found: char },
    InvalidLiteral { index: usize },
    InvalidNumber { index: usize },
    InvalidEscape { index: usize },
    InvalidUnicodeEscape { index: usize },
    InvalidUnicodeScalar { index: usize },
    ExpectedColon { index: usize },
    ExpectedCommaOrEnd { index: usize, context: &'static str },
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteNumber => {
                f.write_str("cannot serialize non-finite floating-point value")
            }
            Self::Io => f.write_str("i/o error while serializing JSON"),
        }
    }
}

impl fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 => f.write_str("input is not valid UTF-8"),
            Self::UnexpectedEnd => f.write_str("unexpected end of JSON input"),
            Self::UnexpectedTrailingCharacters(index) => {
                write!(f, "unexpected trailing characters at byte {index}")
            }
            Self::UnexpectedCharacter { index, found } => {
                write!(f, "unexpected character '{found}' at byte {index}")
            }
            Self::InvalidLiteral { index } => write!(f, "invalid literal at byte {index}"),
            Self::InvalidNumber { index } => write!(f, "invalid number at byte {index}"),
            Self::InvalidEscape { index } => write!(f, "invalid escape sequence at byte {index}"),
            Self::InvalidUnicodeEscape { index } => {
                write!(f, "invalid unicode escape at byte {index}")
            }
            Self::InvalidUnicodeScalar { index } => {
                write!(f, "invalid unicode scalar at byte {index}")
            }
            Self::ExpectedColon { index } => write!(f, "expected ':' at byte {index}"),
            Self::ExpectedCommaOrEnd { index, context } => {
                write!(f, "expected ',' or end of {context} at byte {index}")
            }
        }
    }
}

impl std::error::Error for JsonError {}
impl std::error::Error for JsonParseError {}

impl JsonValue {
    pub fn object(entries: Vec<(impl Into<String>, JsonValue)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    pub fn array(values: Vec<JsonValue>) -> Self {
        Self::Array(values)
    }

    pub fn to_json_string(&self) -> Result<String, JsonError> {
        let mut out = Vec::with_capacity(initial_json_capacity(self));
        write_json_value(&mut out, self)?;
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
            let token = decode_pointer_segment(segment);
            current = match current {
                JsonValue::Object(entries) => entries
                    .iter()
                    .find(|(key, _)| key == &token)
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
            let token = decode_pointer_segment(segment);
            current = match current {
                JsonValue::Object(entries) => entries
                    .iter_mut()
                    .find(|(key, _)| key == &token)
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

impl<'a> BorrowedJsonValue<'a> {
    pub fn into_owned(self) -> JsonValue {
        match self {
            Self::Null => JsonValue::Null,
            Self::Bool(value) => JsonValue::Bool(value),
            Self::Number(value) => JsonValue::Number(value),
            Self::String(value) => JsonValue::String(value.into_owned()),
            Self::Array(values) => JsonValue::Array(
                values
                    .into_iter()
                    .map(BorrowedJsonValue::into_owned)
                    .collect(),
            ),
            Self::Object(entries) => JsonValue::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.into_owned(), value.into_owned()))
                    .collect(),
            ),
        }
    }
}

impl CompiledObjectSchema {
    pub fn new(keys: &[&str]) -> Self {
        let mut fields = Vec::with_capacity(keys.len());
        let mut capacity_hint = 2;
        for (index, key) in keys.iter().enumerate() {
            let mut rendered_prefix = Vec::with_capacity(key.len() + 4);
            if index > 0 {
                rendered_prefix.push(b',');
            }
            write_json_key(&mut rendered_prefix, key);
            capacity_hint += rendered_prefix.len() + 8;
            fields.push(CompiledField {
                key: (*key).to_owned(),
                rendered_prefix,
            });
        }
        Self {
            fields,
            capacity_hint,
        }
    }

    pub fn keys(&self) -> impl ExactSizeIterator<Item = &str> {
        self.fields.iter().map(|field| field.key.as_str())
    }

    pub fn to_json_string<'a, I>(&self, values: I) -> Result<String, JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        let mut out = Vec::with_capacity(self.capacity_hint);
        self.write_json_bytes(&mut out, values)?;
        Ok(unsafe { String::from_utf8_unchecked(out) })
    }

    pub fn write_json_bytes<'a, I>(&self, out: &mut Vec<u8>, values: I) -> Result<(), JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        out.push(b'{');
        let mut iter = values.into_iter();
        for field in &self.fields {
            let Some(value) = iter.next() else {
                panic!(
                    "compiled object schema expected {} values",
                    self.fields.len()
                );
            };
            out.extend_from_slice(&field.rendered_prefix);
            write_json_value(out, value)?;
        }
        if iter.next().is_some() {
            panic!(
                "compiled object schema received more than {} values",
                self.fields.len()
            );
        }
        out.push(b'}');
        Ok(())
    }
}

impl CompiledRowSchema {
    pub fn new(keys: &[&str]) -> Self {
        let object = CompiledObjectSchema::new(keys);
        let row_capacity_hint = object.capacity_hint;
        Self {
            object,
            row_capacity_hint,
        }
    }

    pub fn object_schema(&self) -> &CompiledObjectSchema {
        &self.object
    }

    pub fn to_json_string<'a, R, I>(&self, rows: R) -> Result<String, JsonError>
    where
        R: IntoIterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        let iter = rows.into_iter();
        let (lower, _) = iter.size_hint();
        let mut out = Vec::with_capacity(2 + lower.saturating_mul(self.row_capacity_hint + 1));
        self.write_json_bytes_from_iter(&mut out, iter)?;
        Ok(unsafe { String::from_utf8_unchecked(out) })
    }

    pub fn write_json_bytes<'a, R, I>(&self, out: &mut Vec<u8>, rows: R) -> Result<(), JsonError>
    where
        R: IntoIterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        self.write_json_bytes_from_iter(out, rows.into_iter())
    }

    pub fn write_row_json_bytes<'a, I>(&self, out: &mut Vec<u8>, values: I) -> Result<(), JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        self.object.write_json_bytes(out, values)
    }

    fn write_json_bytes_from_iter<'a, R, I>(
        &self,
        out: &mut Vec<u8>,
        mut rows: R,
    ) -> Result<(), JsonError>
    where
        R: Iterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        out.push(b'[');
        if let Some(first_row) = rows.next() {
            self.object.write_json_bytes(out, first_row)?;
            for row in rows {
                out.push(b',');
                self.object.write_json_bytes(out, row)?;
            }
        }
        out.push(b']');
        Ok(())
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
        Self::Number(JsonNumber::I64(value as i64))
    }
}

impl From<i16> for JsonValue {
    fn from(value: i16) -> Self {
        Self::Number(JsonNumber::I64(value as i64))
    }
}

impl From<i32> for JsonValue {
    fn from(value: i32) -> Self {
        Self::Number(JsonNumber::I64(value as i64))
    }
}

impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        Self::Number(JsonNumber::I64(value))
    }
}

impl From<isize> for JsonValue {
    fn from(value: isize) -> Self {
        Self::Number(JsonNumber::I64(value as i64))
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
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self::Object(
            iter.into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
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

pub fn escape_json_string(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len() + 2);
    write_escaped_json_string(&mut out, input);
    unsafe { String::from_utf8_unchecked(out) }
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


pub fn from_str(input: &str) -> Result<JsonValue, JsonParseError> {
    parse_json(input)
}

pub fn from_slice(input: &[u8]) -> Result<JsonValue, JsonParseError> {
    let input = std::str::from_utf8(input).map_err(|_| JsonParseError::InvalidUtf8)?;
    parse_json(input)
}

pub fn to_string(value: &JsonValue) -> Result<String, JsonError> {
    value.to_json_string()
}

pub fn to_vec(value: &JsonValue) -> Result<Vec<u8>, JsonError> {
    let mut out = Vec::with_capacity(initial_json_capacity(value));
    write_json_value(&mut out, value)?;
    Ok(out)
}

pub fn from_reader<R: Read>(mut reader: R) -> Result<JsonValue, JsonParseError> {
    let mut input = String::new();
    reader
        .read_to_string(&mut input)
        .map_err(|_| JsonParseError::InvalidUtf8)?;
    parse_json(&input)
}

pub fn to_writer<W: Write>(mut writer: W, value: &JsonValue) -> Result<(), JsonError> {
    let bytes = to_vec(value)?;
    writer.write_all(&bytes).map_err(|_| JsonError::Io)
}

pub fn to_string_pretty(value: &JsonValue) -> Result<String, JsonError> {
    let mut out = Vec::with_capacity(initial_json_capacity(value) + 16);
    write_json_value_pretty(&mut out, value, 0)?;
    Ok(unsafe { String::from_utf8_unchecked(out) })
}

pub fn to_vec_pretty(value: &JsonValue) -> Result<Vec<u8>, JsonError> {
    let mut out = Vec::with_capacity(initial_json_capacity(value) + 16);
    write_json_value_pretty(&mut out, value, 0)?;
    Ok(out)
}

pub fn to_writer_pretty<W: Write>(mut writer: W, value: &JsonValue) -> Result<(), JsonError> {
    let bytes = to_vec_pretty(value)?;
    writer.write_all(&bytes).map_err(|_| JsonError::Io)
}

impl JsonTape {
    pub fn root<'a>(&'a self, input: &'a str) -> Option<TapeValue<'a>> {
        (!self.tokens.is_empty()).then_some(TapeValue {
            tape: self,
            input,
            index: 0,
        })
    }
}

impl<'a> TapeValue<'a> {
    pub fn kind(&self) -> TapeTokenKind {
        self.tape.tokens[self.index].kind
    }

    pub fn as_str(&self) -> Option<&'a str> {
        let token = &self.tape.tokens[self.index];
        match token.kind {
            TapeTokenKind::String | TapeTokenKind::Key => {
                if self.input.as_bytes()[token.start] == b'"'
                    && self.input.as_bytes()[token.end - 1] == b'"'
                {
                    Some(&self.input[token.start + 1..token.end - 1])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<TapeValue<'a>> {
        if self.kind() != TapeTokenKind::Object {
            return None;
        }
        self.get_linear(key)
    }

    pub fn build_object_index(&self) -> Option<TapeObjectIndex> {
        if self.kind() != TapeTokenKind::Object {
            return None;
        }
        let parent = self.index;
        let tokens = &self.tape.tokens;
        let mut entries = Vec::new();
        let mut i = self.index + 1;
        while i + 1 < tokens.len() {
            if tokens[i].parent != Some(parent) {
                i += 1;
                continue;
            }
            if tokens[i].kind == TapeTokenKind::Key && tokens[i + 1].parent == Some(parent) {
                let candidate = TapeValue {
                    tape: self.tape,
                    input: self.input,
                    index: i,
                };
                let key = candidate.as_str().unwrap_or("");
                let hash = hash_key(key.as_bytes());
                entries.push((hash, i, i + 1));
                i += 2;
            } else {
                i += 1;
            }
        }
        let bucket_count = (entries.len().next_power_of_two().max(1)) * 2;
        let mut buckets = vec![Vec::new(); bucket_count];
        for entry in entries {
            let bucket = (entry.0 as usize) & (bucket_count - 1);
            buckets[bucket].push(entry);
        }
        Some(TapeObjectIndex { buckets })
    }

    pub fn with_index<'b>(&'b self, index: &'b TapeObjectIndex) -> IndexedTapeObject<'b> {
        IndexedTapeObject {
            object: TapeValue {
                tape: self.tape,
                input: self.input,
                index: self.index,
            },
            index,
        }
    }

    fn get_linear(&self, key: &str) -> Option<TapeValue<'a>> {
        let parent = self.index;
        let tokens = &self.tape.tokens;
        let mut i = self.index + 1;
        while i < tokens.len() {
            if tokens[i].parent != Some(parent) {
                i += 1;
                continue;
            }
            if tokens[i].kind != TapeTokenKind::Key {
                i += 1;
                continue;
            }
            let candidate = TapeValue {
                tape: self.tape,
                input: self.input,
                index: i,
            };
            if candidate.as_str() == Some(key) {
                let value_index = i + 1;
                if value_index < tokens.len() && tokens[value_index].parent == Some(parent) {
                    return Some(TapeValue {
                        tape: self.tape,
                        input: self.input,
                        index: value_index,
                    });
                }
                return None;
            }
            i += 1;
        }
        None
    }
}

impl TapeObjectIndex {
    pub fn get<'a>(&self, object: TapeValue<'a>, key: &str) -> Option<TapeValue<'a>> {
        self.get_hashed(object, hash_key(key.as_bytes()), key)
    }

    pub fn get_compiled<'a>(&self, object: TapeValue<'a>, key: &CompiledTapeKey) -> Option<TapeValue<'a>> {
        self.get_hashed(object, key.hash, &key.key)
    }

    fn get_hashed<'a>(&self, object: TapeValue<'a>, hash: u64, key: &str) -> Option<TapeValue<'a>> {
        let bucket = (hash as usize) & (self.buckets.len() - 1);
        for (entry_hash, key_index, value_index) in &self.buckets[bucket] {
            if *entry_hash != hash {
                continue;
            }
            let candidate = TapeValue {
                tape: object.tape,
                input: object.input,
                index: *key_index,
            };
            if candidate.as_str() == Some(key) {
                return Some(TapeValue {
                    tape: object.tape,
                    input: object.input,
                    index: *value_index,
                });
            }
        }
        None
    }
}

impl CompiledTapeKey {
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        let hash = hash_key(key.as_bytes());
        Self { key, hash }
    }

    pub fn as_str(&self) -> &str {
        &self.key
    }
}

impl CompiledTapeKeys {
    pub fn new(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|key| CompiledTapeKey::new(*key)).collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &CompiledTapeKey> {
        self.keys.iter()
    }
}

impl<'a> IndexedTapeObject<'a> {
    pub fn get(&self, key: &str) -> Option<TapeValue<'a>> {
        self.index.get(self.object, key)
    }

    pub fn get_compiled(&self, key: &CompiledTapeKey) -> Option<TapeValue<'a>> {
        self.index.get_compiled(self.object, key)
    }

    pub fn get_many<'b>(
        &'b self,
        keys: &'b [&'b str],
    ) -> impl Iterator<Item = Option<TapeValue<'a>>> + 'b {
        keys.iter().map(|key| self.get(key))
    }

    pub fn get_compiled_many<'b>(
        &'b self,
        keys: &'b CompiledTapeKeys,
    ) -> impl Iterator<Item = Option<TapeValue<'a>>> + 'b {
        keys.iter().map(|key| self.get_compiled(key))
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

static JSON_NULL: JsonValue = JsonValue::Null;

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

fn object_get<'a>(entries: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    entries
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value)
}

fn object_get_mut<'a>(entries: &'a mut Vec<(String, JsonValue)>, key: &str) -> Option<&'a mut JsonValue> {
    entries
        .iter_mut()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value)
}

fn object_index_or_insert<'a>(value: &'a mut JsonValue, key: &str) -> &'a mut JsonValue {
    if matches!(value, JsonValue::Null) {
        *value = JsonValue::Object(Vec::new());
    }
    match value {
        JsonValue::Object(entries) => {
            if let Some(pos) = entries.iter().position(|(candidate, _)| candidate == key) {
                &mut entries[pos].1
            } else {
                entries.push((key.to_owned(), JsonValue::Null));
                &mut entries.last_mut().unwrap().1
            }
        }
        JsonValue::Null => unreachable!(),
        JsonValue::Bool(_) => panic!("cannot access key {:?} in JSON boolean", key),
        JsonValue::Number(_) => panic!("cannot access key {:?} in JSON number", key),
        JsonValue::String(_) => panic!("cannot access key {:?} in JSON string", key),
        JsonValue::Array(_) => panic!("cannot access key {:?} in JSON array", key),
    }
}

fn array_index_or_panic(value: &mut JsonValue, index: usize) -> &mut JsonValue {
    match value {
        JsonValue::Array(values) => {
            let len = values.len();
            values.get_mut(index).unwrap_or_else(|| panic!("cannot access index {} of JSON array of length {}", index, len))
        }
        JsonValue::Null => panic!("cannot access index {} of JSON null", index),
        JsonValue::Bool(_) => panic!("cannot access index {} of JSON boolean", index),
        JsonValue::Number(_) => panic!("cannot access index {} of JSON number", index),
        JsonValue::String(_) => panic!("cannot access index {} of JSON string", index),
        JsonValue::Object(_) => panic!("cannot access index {} of JSON object", index),
    }
}

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

#[macro_export]
macro_rules! json {
    (null) => {
        $crate::JsonValue::Null
    };
    ([$($element:tt),* $(,)?]) => {
        $crate::JsonValue::Array(vec![$($crate::json!($element)),*])
    };
    ({$($key:literal : $value:tt),* $(,)?}) => {
        $crate::JsonValue::Object(vec![$(($key.to_owned(), $crate::json!($value))),*])
    };
    ($other:expr) => {
        $crate::JsonValue::from($other)
    };
}

fn decode_pointer_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    let bytes = segment.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'~' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'0' => {
                    out.push('~');
                    i += 2;
                    continue;
                }
                b'1' => {
                    out.push('/');
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn write_indent(out: &mut Vec<u8>, depth: usize) {
    for _ in 0..depth {
        out.extend_from_slice(b"  ");
    }
}

fn write_json_value_pretty(out: &mut Vec<u8>, value: &JsonValue, depth: usize) -> Result<(), JsonError> {
    match value {
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_) => {
            write_json_value(out, value)
        }
        JsonValue::Array(values) => {
            out.push(b'[');
            if !values.is_empty() {
                out.push(b'\n');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.extend_from_slice(b",\n");
                    }
                    write_indent(out, depth + 1);
                    write_json_value_pretty(out, value, depth + 1)?;
                }
                out.push(b'\n');
                write_indent(out, depth);
            }
            out.push(b']');
            Ok(())
        }
        JsonValue::Object(entries) => {
            out.push(b'{');
            if !entries.is_empty() {
                out.push(b'\n');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.extend_from_slice(b",\n");
                    }
                    write_indent(out, depth + 1);
                    write_json_key(out, key);
                    out.push(b' ');
                    write_json_value_pretty(out, value, depth + 1)?;
                }
                out.push(b'\n');
                write_indent(out, depth);
            }
            out.push(b'}');
            Ok(())
        }
    }
}

fn hash_key(bytes: &[u8]) -> u64 {
    let mut hash = 1469598103934665603u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

#[inline]
fn write_json_value(out: &mut Vec<u8>, value: &JsonValue) -> Result<(), JsonError> {
    match value {
        JsonValue::Null => out.extend_from_slice(b"null"),
        JsonValue::Bool(value) => {
            if *value {
                out.extend_from_slice(b"true");
            } else {
                out.extend_from_slice(b"false");
            }
        }
        JsonValue::Number(number) => write_json_number(out, number)?,
        JsonValue::String(value) => {
            write_escaped_json_string(out, value);
        }
        JsonValue::Array(values) => {
            write_json_array(out, values)?;
        }
        JsonValue::Object(entries) => {
            write_json_object(out, entries)?;
        }
    }
    Ok(())
}

#[inline]
fn write_json_number(out: &mut Vec<u8>, value: &JsonNumber) -> Result<(), JsonError> {
    match value {
        JsonNumber::I64(value) => {
            append_i64(out, *value);
            Ok(())
        }
        JsonNumber::U64(value) => {
            append_u64(out, *value);
            Ok(())
        }
        JsonNumber::F64(value) => {
            if !value.is_finite() {
                return Err(JsonError::NonFiniteNumber);
            }
            out.extend_from_slice(value.to_string().as_bytes());
            Ok(())
        }
    }
}

#[inline]
fn write_escaped_json_string(out: &mut Vec<u8>, input: &str) {
    out.push(b'"');
    let bytes = input.as_bytes();
    let mut fast_index = 0usize;
    while fast_index < bytes.len() {
        let byte = bytes[fast_index];
        if needs_escape(byte) {
            break;
        }
        fast_index += 1;
    }
    if fast_index == bytes.len() {
        out.extend_from_slice(bytes);
        out.push(b'"');
        return;
    }

    if fast_index > 0 {
        out.extend_from_slice(&bytes[..fast_index]);
    }

    let mut chunk_start = fast_index;
    for (index, byte) in bytes.iter().copied().enumerate().skip(fast_index) {
        let escape = match byte {
            b'"' => Some(br#"\""#.as_slice()),
            b'\\' => Some(br#"\\"#.as_slice()),
            0x08 => Some(br#"\b"#.as_slice()),
            0x0c => Some(br#"\f"#.as_slice()),
            b'\n' => Some(br#"\n"#.as_slice()),
            b'\r' => Some(br#"\r"#.as_slice()),
            b'\t' => Some(br#"\t"#.as_slice()),
            _ => None,
        };
        if let Some(escape) = escape {
            if chunk_start < index {
                out.extend_from_slice(&bytes[chunk_start..index]);
            }
            out.extend_from_slice(escape);
            chunk_start = index + 1;
            continue;
        }
        if byte <= 0x1f {
            if chunk_start < index {
                out.extend_from_slice(&bytes[chunk_start..index]);
            }
            out.extend_from_slice(br#"\u00"#);
            out.push(hex_digit((byte >> 4) & 0x0f));
            out.push(hex_digit(byte & 0x0f));
            chunk_start = index + 1;
        }
    }
    if chunk_start < input.len() {
        out.extend_from_slice(&bytes[chunk_start..]);
    }
    out.push(b'"');
}

#[inline]
fn needs_escape(byte: u8) -> bool {
    matches!(byte, b'"' | b'\\' | 0x00..=0x1f)
}

#[inline]
fn write_json_array(out: &mut Vec<u8>, values: &[JsonValue]) -> Result<(), JsonError> {
    out.push(b'[');
    match values {
        [] => {}
        [one] => {
            write_json_value(out, one)?;
        }
        [a, b] => {
            write_json_value(out, a)?;
            out.push(b',');
            write_json_value(out, b)?;
        }
        [a, b, c] => {
            write_json_value(out, a)?;
            out.push(b',');
            write_json_value(out, b)?;
            out.push(b',');
            write_json_value(out, c)?;
        }
        _ => {
            let mut iter = values.iter();
            if let Some(first) = iter.next() {
                write_json_value(out, first)?;
                for value in iter {
                    out.push(b',');
                    write_json_value(out, value)?;
                }
            }
        }
    }
    out.push(b']');
    Ok(())
}

#[inline]
fn write_json_object(out: &mut Vec<u8>, entries: &[(String, JsonValue)]) -> Result<(), JsonError> {
    out.push(b'{');
    match entries {
        [] => {}
        [(k1, v1)] => {
            write_json_key(out, k1);
            write_json_value(out, v1)?;
        }
        [(k1, v1), (k2, v2)] => {
            write_json_key(out, k1);
            write_json_value(out, v1)?;
            out.push(b',');
            write_json_key(out, k2);
            write_json_value(out, v2)?;
        }
        [(k1, v1), (k2, v2), (k3, v3)] => {
            write_json_key(out, k1);
            write_json_value(out, v1)?;
            out.push(b',');
            write_json_key(out, k2);
            write_json_value(out, v2)?;
            out.push(b',');
            write_json_key(out, k3);
            write_json_value(out, v3)?;
        }
        _ => {
            let mut iter = entries.iter();
            if let Some((first_key, first_value)) = iter.next() {
                write_json_key(out, first_key);
                write_json_value(out, first_value)?;
                for (key, value) in iter {
                    out.push(b',');
                    write_json_key(out, key);
                    write_json_value(out, value)?;
                }
            }
        }
    }
    out.push(b'}');
    Ok(())
}

#[inline]
fn write_json_key(out: &mut Vec<u8>, key: &str) {
    let bytes = key.as_bytes();
    if is_plain_json_string(bytes) {
        out.push(b'"');
        out.extend_from_slice(bytes);
        out.extend_from_slice(b"\":");
    } else {
        write_escaped_json_string(out, key);
        out.push(b':');
    }
}

#[inline]
fn is_plain_json_string(bytes: &[u8]) -> bool {
    for &byte in bytes {
        if needs_escape(byte) {
            return false;
        }
    }
    true
}

fn initial_json_capacity(value: &JsonValue) -> usize {
    match value {
        JsonValue::Null => 4,
        JsonValue::Bool(true) => 4,
        JsonValue::Bool(false) => 5,
        JsonValue::Number(JsonNumber::I64(value)) => estimate_i64_len(*value),
        JsonValue::Number(JsonNumber::U64(value)) => estimate_u64_len(*value),
        JsonValue::Number(JsonNumber::F64(_)) => 24,
        JsonValue::String(value) => estimate_escaped_string_len(value),
        JsonValue::Array(values) => 2 + values.len().saturating_mul(16),
        JsonValue::Object(entries) => {
            2 + entries
                .iter()
                .map(|(key, _)| estimate_escaped_string_len(key) + 8)
                .sum::<usize>()
        }
    }
}

fn estimate_escaped_string_len(value: &str) -> usize {
    let mut len = 2;
    for ch in value.chars() {
        len += match ch {
            '"' | '\\' | '\u{08}' | '\u{0C}' | '\n' | '\r' | '\t' => 2,
            ch if ch <= '\u{1F}' => 6,
            ch => ch.len_utf8(),
        };
    }
    len
}

fn estimate_u64_len(mut value: u64) -> usize {
    let mut len = 1;
    while value >= 10 {
        value /= 10;
        len += 1;
    }
    len
}

fn estimate_i64_len(value: i64) -> usize {
    if value < 0 {
        1 + estimate_u64_len(value.unsigned_abs())
    } else {
        estimate_u64_len(value as u64)
    }
}

fn append_i64(out: &mut Vec<u8>, value: i64) {
    if value < 0 {
        out.push(b'-');
        append_u64(out, value.unsigned_abs());
    } else {
        append_u64(out, value as u64);
    }
}

fn append_u64(out: &mut Vec<u8>, mut value: u64) {
    let mut buf = [0u8; 20];
    let mut index = buf.len();
    loop {
        index -= 1;
        buf[index] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    out.extend_from_slice(&buf[index..]);
}

fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        10..=15 => b'a' + (value - 10),
        _ => unreachable!(),
    }
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            index: 0,
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonParseError> {
        self.skip_whitespace();
        match self.peek_byte() {
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
        }
    }

    fn parse_value_borrowed(&mut self) -> Result<BorrowedJsonValue<'a>, JsonParseError> {
        self.skip_whitespace();
        match self.peek_byte() {
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
        }
    }

    fn parse_tape_value(
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
        Ok(token_index)
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonParseError> {
        self.consume_byte(b'{')?;
        self.skip_whitespace();
        let mut entries = Vec::new();
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
                b'0'..=b'9' => (ch - b'0') as u32,
                b'a'..=b'f' => 10 + (ch - b'a') as u32,
                b'A'..=b'F' => 10 + (ch - b'A') as u32,
                _ => return Err(JsonParseError::InvalidUnicodeEscape { index }),
            };
            value = (value << 4) | digit;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonNumber, JsonParseError> {
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
            let value = token
                .parse::<i64>()
                .map_err(|_| JsonParseError::InvalidNumber { index: start })?;
            Ok(JsonNumber::I64(value))
        } else {
            let value = token
                .parse::<u64>()
                .map_err(|_| JsonParseError::InvalidNumber { index: start })?;
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

    fn skip_whitespace(&mut self) {
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

    fn is_eof(&self) -> bool {
        self.index >= self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_control_characters_and_quotes() {
        let escaped = escape_json_string("hello\t\"world\"\n\u{0007}");
        assert_eq!(escaped, "\"hello\\t\\\"world\\\"\\n\\u0007\"");
    }

    #[test]
    fn serializes_nested_values() {
        let value = JsonValue::object(vec![
            ("name", "node-1".into()),
            ("ok", true.into()),
            (
                "values",
                JsonValue::array(vec![1u32.into(), 2u32.into(), JsonValue::Null]),
            ),
        ]);
        assert_eq!(
            value.to_json_string().unwrap(),
            "{\"name\":\"node-1\",\"ok\":true,\"values\":[1,2,null]}"
        );
    }

    #[test]
    fn rejects_non_finite_float() {
        let value = JsonValue::from(f64::NAN);
        assert_eq!(value.to_json_string(), Err(JsonError::NonFiniteNumber));
    }

    #[test]
    fn parses_basic_json_values() {
        assert_eq!(parse_json("null").unwrap(), JsonValue::Null);
        assert_eq!(parse_json("true").unwrap(), JsonValue::Bool(true));
        assert_eq!(
            parse_json("\"hello\"").unwrap(),
            JsonValue::String("hello".into())
        );
        assert_eq!(
            parse_json("123").unwrap(),
            JsonValue::Number(JsonNumber::U64(123))
        );
        assert_eq!(
            parse_json("-123").unwrap(),
            JsonValue::Number(JsonNumber::I64(-123))
        );
    }

    #[test]
    fn parses_unicode_and_escapes() {
        let value = parse_json("\"line\\n\\u03bb\\uD83D\\uDE80\"").unwrap();
        assert_eq!(value, JsonValue::String("line\nλ🚀".into()));
    }

    #[test]
    fn borrowed_parse_avoids_allocating_plain_strings() {
        let value = parse_json_borrowed("{\"name\":\"hello\",\"n\":1}").unwrap();
        match value {
            BorrowedJsonValue::Object(entries) => {
                assert!(matches!(entries[0].0, Cow::Borrowed(_)));
                assert!(matches!(
                    entries[0].1,
                    BorrowedJsonValue::String(Cow::Borrowed(_))
                ));
            }
            other => panic!("unexpected value: {other:?}"),
        }
    }

    #[test]
    fn borrowed_parse_allocates_when_unescaping_is_needed() {
        let value = parse_json_borrowed("\"line\\nvalue\"").unwrap();
        match value {
            BorrowedJsonValue::String(Cow::Owned(text)) => assert_eq!(text, "line\nvalue"),
            other => panic!("unexpected value: {other:?}"),
        }
    }

    #[test]
    fn compiled_schema_serializes_expected_shape() {
        let schema = CompiledObjectSchema::new(&["id", "name", "enabled"]);
        let values = [
            JsonValue::from(7u64),
            JsonValue::from("node-7"),
            JsonValue::from(true),
        ];
        let json = schema.to_json_string(values.iter()).unwrap();
        assert_eq!(json, "{\"id\":7,\"name\":\"node-7\",\"enabled\":true}");
    }

    #[test]
    fn compiled_row_schema_serializes_array_of_objects() {
        let schema = CompiledRowSchema::new(&["id", "name"]);
        let row1 = [JsonValue::from(1u64), JsonValue::from("a")];
        let row2 = [JsonValue::from(2u64), JsonValue::from("b")];
        let json = schema.to_json_string([row1.iter(), row2.iter()]).unwrap();
        assert_eq!(json, r#"[{"id":1,"name":"a"},{"id":2,"name":"b"}]"#);
    }

    #[test]
    fn tape_parse_records_structure_tokens() {
        let tape = parse_json_tape(r#"{"a":[1,"x"],"b":true}"#).unwrap();
        assert_eq!(tape.tokens[0].kind, TapeTokenKind::Object);
        assert_eq!(tape.tokens[1].kind, TapeTokenKind::Key);
        assert_eq!(tape.tokens[2].kind, TapeTokenKind::Array);
        assert_eq!(tape.tokens[3].kind, TapeTokenKind::Number);
        assert_eq!(tape.tokens[4].kind, TapeTokenKind::String);
        assert_eq!(tape.tokens[5].kind, TapeTokenKind::Key);
        assert_eq!(tape.tokens[6].kind, TapeTokenKind::Bool);
    }

    #[test]
    fn tape_object_lookup_finds_child_values() {
        let input = r#"{"name":"hello","nested":{"x":1},"flag":true}"#;
        let tape = parse_json_tape(input).unwrap();
        let root = tape.root(input).unwrap();
        let name = root.get("name").unwrap();
        assert_eq!(name.kind(), TapeTokenKind::String);
        assert_eq!(name.as_str(), Some("hello"));
        let nested = root.get("nested").unwrap();
        assert_eq!(nested.kind(), TapeTokenKind::Object);
        assert!(root.get("missing").is_none());
    }

    #[test]
    fn tape_object_index_lookup_finds_child_values() {
        let input = r#"{"name":"hello","nested":{"x":1},"flag":true}"#;
        let tape = parse_json_tape(input).unwrap();
        let root = tape.root(input).unwrap();
        let index = root.build_object_index().unwrap();
        let flag = index.get(root, "flag").unwrap();
        assert_eq!(flag.kind(), TapeTokenKind::Bool);
        assert!(index.get(root, "missing").is_none());
    }

    #[test]
    fn indexed_tape_object_compiled_lookup_finds_child_values() {
        let input = r#"{"name":"hello","nested":{"x":1},"flag":true}"#;
        let tape = parse_json_tape(input).unwrap();
        let root = tape.root(input).unwrap();
        let index = root.build_object_index().unwrap();
        let indexed = root.with_index(&index);
        let keys = CompiledTapeKeys::new(&["name", "flag", "missing"]);
        let got = indexed
            .get_compiled_many(&keys)
            .map(|value| value.map(|value| value.kind()))
            .collect::<Vec<_>>();
        assert_eq!(got, vec![Some(TapeTokenKind::String), Some(TapeTokenKind::Bool), None]);
    }

    #[test]
    fn serde_style_convenience_api_works() {
        let value = from_str(r#"{"ok":true,"n":7,"items":[1,2,3],"msg":"hello"}"#).unwrap();
        assert!(value.is_object());
        assert_eq!(value["ok"].as_bool(), Some(true));
        assert_eq!(value["n"].as_i64(), Some(7));
        assert_eq!(value["msg"].as_str(), Some("hello"));
        assert_eq!(value["items"][1].as_u64(), Some(2));
        assert!(value["missing"].is_null());
        assert_eq!(to_string(&value).unwrap(), r#"{"ok":true,"n":7,"items":[1,2,3],"msg":"hello"}"#);
        assert_eq!(from_slice(br#"[1,true,"x"]"#).unwrap()[2].as_str(), Some("x"));
        assert_eq!(to_vec(&value).unwrap(), value.to_json_string().unwrap().into_bytes());
    }

    #[test]
    fn json_macro_builds_values() {
        let value = json!({"ok": true, "items": [1, 2, null], "msg": "x"});
        assert_eq!(value["ok"].as_bool(), Some(true));
        assert_eq!(value["items"][0].as_u64(), Some(1));
        assert!(value["items"][2].is_null());
        assert_eq!(value["msg"].as_str(), Some("x"));
    }

    #[test]
    fn from_slice_rejects_invalid_utf8() {
        assert!(matches!(from_slice(&[0xff]), Err(JsonParseError::InvalidUtf8)));
    }

    #[test]
    fn pointer_take_and_pretty_helpers_work() {
        let mut value = from_str(r#"{"a":{"b":[10,20,{"~key/":"x"}]}}"#).unwrap();
        assert_eq!(value.pointer("/a/b/1").and_then(JsonValue::as_u64), Some(20));
        assert_eq!(value.pointer("/a/b/2/~0key~1").and_then(JsonValue::as_str), Some("x"));
        *value.pointer_mut("/a/b/0").unwrap() = JsonValue::from(99u64);
        assert_eq!(value.pointer("/a/b/0").and_then(JsonValue::as_u64), Some(99));

        let taken = value.pointer_mut("/a/b/2").unwrap().take();
        assert!(value.pointer("/a/b/2").unwrap().is_null());
        assert_eq!(taken["~key/"].as_str(), Some("x"));

        let pretty = to_string_pretty(&value).unwrap();
        assert!(pretty.contains("\"a\": {"));
        let mut out = Vec::new();
        to_writer_pretty(&mut out, &value).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), pretty);
    }

    #[test]
    fn reader_writer_and_collection_helpers_work() {
        let value = from_reader(std::io::Cursor::new(br#"{"a":1,"b":[true,false]}"# as &[u8])).unwrap();
        assert_eq!(value["a"].as_u64(), Some(1));
        assert_eq!(value["b"].len(), 2);
        assert_eq!(value["b"].get_index(1).and_then(JsonValue::as_bool), Some(false));

        let mut out = Vec::new();
        to_writer(&mut out, &value).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), value.to_json_string().unwrap());

        let object = JsonValue::from_iter([("x", 1u64), ("y", 2u64)]);
        assert_eq!(object["x"].as_u64(), Some(1));
        let array = JsonValue::from_iter([1u64, 2u64, 3u64]);
        assert_eq!(array.get_index(2).and_then(JsonValue::as_u64), Some(3));
    }

    #[test]
    fn signature_and_sort_parity_helpers_work() {
        let mut value = json!({"z": {"b": 2, "a": 1}, "a": [{"d": 4, "c": 3}]});
        assert_eq!(value.as_object().unwrap().len(), 2);
        assert_eq!(value["a"].as_array().unwrap().len(), 1);
        value.sort_all_objects();
        let root_keys = value.as_object().unwrap().iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>();
        assert_eq!(root_keys, vec!["a", "z"]);
        let nested_keys = value["z"].as_object().unwrap().iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>();
        assert_eq!(nested_keys, vec!["a", "b"]);
    }

    #[test]
    fn generic_get_and_get_mut_index_parity_work() {
        let mut value = json!({"obj": {"x": 1}, "arr": [10, 20, 30]});
        let key = String::from("obj");
        assert_eq!(value.get("obj").and_then(|v| v.get("x")).and_then(JsonValue::as_u64), Some(1));
        assert_eq!(value.get(&key).and_then(|v| v.get("x")).and_then(JsonValue::as_u64), Some(1));
        assert_eq!(value.get("arr").and_then(|v| v.get(1)).and_then(JsonValue::as_u64), Some(20));
        *value.get_mut("arr").unwrap().get_mut(2).unwrap() = JsonValue::from(99u64);
        assert_eq!(value["arr"][2].as_u64(), Some(99));
    }

    #[test]
    fn number_and_mut_index_parity_helpers_work() {
        let int = JsonValue::from(7i64);
        assert!(int.is_i64());
        assert!(!int.is_u64());
        assert!(!int.is_f64());
        assert_eq!(int.as_number().and_then(JsonNumber::as_i64), Some(7));

        let float = JsonValue::Number(JsonNumber::from_f64(2.5).unwrap());
        assert!(float.is_f64());
        assert_eq!(float.as_f64(), Some(2.5));
        assert_eq!(JsonValue::Null.as_null(), Some(()));

        let mut value = JsonValue::Null;
        value["a"]["b"]["c"] = JsonValue::from(true);
        assert_eq!(value.pointer("/a/b/c").and_then(JsonValue::as_bool), Some(true));

        value["arr"] = json!([1, 2, 3]);
        value["arr"][1] = JsonValue::from(9u64);
        assert_eq!(value.pointer("/arr/1").and_then(JsonValue::as_u64), Some(9));
    }

    #[test]
    fn rejects_invalid_json_inputs() {
        assert!(matches!(
            parse_json("{"),
            Err(JsonParseError::UnexpectedEnd)
        ));
        assert!(matches!(
            parse_json("{\"a\" 1}"),
            Err(JsonParseError::ExpectedColon { .. })
        ));
        assert!(matches!(
            parse_json("[1 2]"),
            Err(JsonParseError::ExpectedCommaOrEnd {
                context: "array",
                ..
            })
        ));
        assert!(matches!(
            parse_json("{\"a\":1 trailing"),
            Err(JsonParseError::ExpectedCommaOrEnd {
                context: "object",
                ..
            })
        ));
        assert!(matches!(
            parse_json("00"),
            Err(JsonParseError::InvalidNumber { .. })
        ));
    }

    #[test]
    fn roundtrips_specific_structures() {
        let values = [
            JsonValue::Null,
            JsonValue::Bool(false),
            JsonValue::String("tab\tquote\"slash\\snowman☃".into()),
            JsonValue::Number(JsonNumber::I64(-9_223_372_036_854_775_808)),
            JsonValue::Number(JsonNumber::U64(u64::MAX)),
            JsonValue::Number(JsonNumber::F64(12345.125)),
            JsonValue::Array(vec![
                JsonValue::Bool(true),
                JsonValue::String("nested".into()),
                JsonValue::Object(vec![("x".into(), 1u64.into())]),
            ]),
        ];
        for value in values {
            let text = value.to_json_string().unwrap();
            let reparsed = parse_json(&text).unwrap();
            assert_json_equivalent(&value, &reparsed);
        }
    }

    #[test]
    fn deterministic_fuzz_roundtrip_strings_and_values() {
        let mut rng = Rng::new(0x5eed_1234_5678_9abc);
        for _ in 0..2_000 {
            let input = random_string(&mut rng, 48);
            let escaped = escape_json_string(&input);
            let parsed = parse_json(&escaped).unwrap();
            assert_eq!(parsed, JsonValue::String(input));
        }

        for _ in 0..1_000 {
            let value = random_json_value(&mut rng, 0, 4);
            let text = value.to_json_string().unwrap();
            let reparsed = parse_json(&text).unwrap();
            assert_json_equivalent(&value, &reparsed);
        }
    }

    fn assert_json_equivalent(expected: &JsonValue, actual: &JsonValue) {
        match (expected, actual) {
            (JsonValue::Null, JsonValue::Null) => {}
            (JsonValue::Bool(a), JsonValue::Bool(b)) => assert_eq!(a, b),
            (JsonValue::String(a), JsonValue::String(b)) => assert_eq!(a, b),
            (JsonValue::Number(a), JsonValue::Number(b)) => assert_numbers_equivalent(a, b),
            (JsonValue::Array(a), JsonValue::Array(b)) => {
                assert_eq!(a.len(), b.len());
                for (left, right) in a.iter().zip(b.iter()) {
                    assert_json_equivalent(left, right);
                }
            }
            (JsonValue::Object(a), JsonValue::Object(b)) => {
                assert_eq!(a.len(), b.len());
                for ((left_key, left_value), (right_key, right_value)) in a.iter().zip(b.iter()) {
                    assert_eq!(left_key, right_key);
                    assert_json_equivalent(left_value, right_value);
                }
            }
            _ => panic!("json values differ: expected {expected:?}, actual {actual:?}"),
        }
    }

    fn assert_numbers_equivalent(expected: &JsonNumber, actual: &JsonNumber) {
        match (expected, actual) {
            (JsonNumber::I64(a), JsonNumber::I64(b)) => assert_eq!(a, b),
            (JsonNumber::U64(a), JsonNumber::U64(b)) => assert_eq!(a, b),
            (JsonNumber::F64(a), JsonNumber::F64(b)) => assert_eq!(a.to_bits(), b.to_bits()),
            (JsonNumber::I64(a), JsonNumber::U64(b)) if *a >= 0 => assert_eq!(*a as u64, *b),
            (JsonNumber::U64(a), JsonNumber::I64(b)) if *b >= 0 => assert_eq!(*a, *b as u64),
            (JsonNumber::I64(a), JsonNumber::F64(b)) => assert_eq!(*a as f64, *b),
            (JsonNumber::U64(a), JsonNumber::F64(b)) => assert_eq!(*a as f64, *b),
            (JsonNumber::F64(a), JsonNumber::I64(b)) => assert_eq!(*a, *b as f64),
            (JsonNumber::F64(a), JsonNumber::U64(b)) => assert_eq!(*a, *b as f64),
            (left, right) => panic!("json numbers differ: expected {left:?}, actual {right:?}"),
        }
    }

    #[derive(Clone, Debug)]
    struct Rng {
        state: u64,
    }

    impl Rng {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next_u64(&mut self) -> u64 {
            self.state = self
                .state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.state
        }

        fn choose(&mut self, upper_exclusive: usize) -> usize {
            (self.next_u64() % upper_exclusive as u64) as usize
        }

        fn bool(&mut self) -> bool {
            (self.next_u64() & 1) == 1
        }
    }

    fn random_string(rng: &mut Rng, max_len: usize) -> String {
        let len = rng.choose(max_len + 1);
        let mut out = String::new();
        for _ in 0..len {
            let ch = match rng.choose(12) {
                0 => '"',
                1 => '\\',
                2 => '\n',
                3 => '\r',
                4 => '\t',
                5 => '\u{0007}',
                6 => 'λ',
                7 => '🚀',
                8 => '☃',
                _ => (b'a' + rng.choose(26) as u8) as char,
            };
            out.push(ch);
        }
        out
    }

    fn random_json_value(rng: &mut Rng, depth: usize, max_depth: usize) -> JsonValue {
        if depth >= max_depth {
            return random_leaf(rng);
        }
        match rng.choose(7) {
            0 | 1 | 2 | 3 => random_leaf(rng),
            4 => {
                let len = rng.choose(5);
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(random_json_value(rng, depth + 1, max_depth));
                }
                JsonValue::Array(values)
            }
            _ => {
                let len = rng.choose(5);
                let mut entries = Vec::with_capacity(len);
                for index in 0..len {
                    entries.push((
                        format!("k{depth}_{index}_{}", random_string(rng, 6)),
                        random_json_value(rng, depth + 1, max_depth),
                    ));
                }
                JsonValue::Object(entries)
            }
        }
    }

    fn random_leaf(rng: &mut Rng) -> JsonValue {
        match rng.choose(6) {
            0 => JsonValue::Null,
            1 => JsonValue::Bool(rng.bool()),
            2 => JsonValue::String(random_string(rng, 24)),
            3 => JsonValue::Number(JsonNumber::I64(
                (rng.next_u64() >> 1) as i64 * if rng.bool() { 1 } else { -1 },
            )),
            4 => JsonValue::Number(JsonNumber::U64(rng.next_u64())),
            _ => {
                let mantissa = (rng.next_u64() % 1_000_000) as f64 / 1000.0;
                let sign = if rng.bool() { 1.0 } else { -1.0 };
                JsonValue::Number(JsonNumber::F64(sign * mantissa))
            }
        }
    }
}
