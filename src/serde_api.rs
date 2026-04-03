//! High-level JSON parsing and serialization API.
//!
//! This module provides the main entry points for working with JSON data.
//! The same function names work with both owned [`JsonValue`] and typed serde
//! deserialization (when the `serde` feature is enabled).
//!
//! # Parsing
//!
//! - [`parse_json`] — parse into an owned [`JsonValue`]
//! - [`parse_json_borrowed`] — parse with zero-copy string borrowing
//! - [`parse_json_tape`] — parse into a token tape for fast indexed access
//! - [`from_str`] / [`from_slice`] — serde-compatible parsing
//!
//! # Serialization
//!
//! - [`to_string`] / [`to_vec`] — compact serialization
//! - [`to_string_pretty`] / [`to_vec_pretty`] — pretty (indented) serialization
//! - [`to_writer`] / [`to_writer_pretty`] — write to any [`Write`] implementation
//! - [`from_reader`] — read from any [`Read`] implementation
//!
//! # Serde Integration (feature-gated)
//!
//! When the `serde` feature is enabled, the same functions support typed
//! deserialization. See the [crate-level documentation](crate) for examples.

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(not(feature = "serde"))]
use crate::error::JsonError;
use crate::error::JsonParseError;
use crate::util;
use crate::JsonValue;
#[cfg(feature = "std")]
use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Escapes a string as a JSON string literal (including surrounding quotes).
///
/// # Example
///
/// ```
/// use lifegraph_json::escape_json_string;
///
/// let escaped = escape_json_string("hello\tworld");
/// assert_eq!(escaped, r#""hello\tworld""#);
/// ```
pub fn escape_json_string(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len() + 2);
    util::write_escaped_json_string(&mut out, input);
    unsafe { String::from_utf8_unchecked(out) }
}

/// Parses a JSON string into an owned [`JsonValue`].
///
/// # Example
///
/// ```
/// use lifegraph_json::parse_json;
///
/// let value = parse_json(r#"{"name":"Alice","scores":[95,87]}"#)?;
/// assert_eq!(value["name"].as_str(), Some("Alice"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns a [`JsonParseError`] if the input is not valid JSON.
pub fn parse_json(input: &str) -> Result<JsonValue, JsonParseError> {
    let mut parser = crate::parse::Parser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(value)
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index()))
    }
}

/// Parses a JSON string with zero-copy borrowing of plain strings and keys.
///
/// This avoids allocating [`String`]s for string values and object keys
/// when no unescaping is needed. The result borrows from the input.
///
/// # Example
///
/// ```
/// use lifegraph_json::parse_json_borrowed;
///
/// let input = r#"{"name":"Alice","active":true}"#;
/// let value = parse_json_borrowed(input)?;
/// // Convert to owned for easy inspection
/// let owned = value.into_owned();
/// assert_eq!(owned["name"].as_str(), Some("Alice"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns a [`JsonParseError`] if the input is not valid JSON.
pub fn parse_json_borrowed(
    input: &str,
) -> Result<crate::borrowed_value::BorrowedJsonValue<'_>, JsonParseError> {
    let mut parser = crate::parse::Parser::new(input);
    let value = parser.parse_value_borrowed()?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(value)
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index()))
    }
}

/// Parses a JSON string into a token tape for fast indexed access.
///
/// The tape is a flat array of tokens with parent pointers, enabling
/// efficient random access without recursive traversal.
///
/// # Example
///
/// ```
/// use lifegraph_json::parse_json_tape;
///
/// let tape = parse_json_tape(r#"{"users":[{"name":"Alice"},{"name":"Bob"}]}"#)?;
/// let root = tape.root(r#"{"users":[{"name":"Alice"},{"name":"Bob"}]}"#).unwrap();
/// let users = root.get("users").unwrap();
/// assert_eq!(users.kind(), lifegraph_json::TapeTokenKind::Array);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns a [`JsonParseError`] if the input is not valid JSON.
pub fn parse_json_tape(input: &str) -> Result<crate::tape::JsonTape, JsonParseError> {
    let mut parser = crate::parse::Parser::new(input);
    let mut tokens = Vec::new();
    parser.parse_tape_value(&mut tokens, None)?;
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(crate::tape::JsonTape { tokens })
    } else {
        Err(JsonParseError::UnexpectedTrailingCharacters(parser.index()))
    }
}

// ---------------------------------------------------------------------------
// Serde convenience functions (serde feature gate)
// ---------------------------------------------------------------------------

/// Converts a serializable value into a [`JsonValue`].
///
/// # Feature
///
/// Requires the `serde` feature flag.
#[cfg(feature = "serde")]
pub fn to_value<T>(value: T) -> Result<JsonValue, crate::serde_error::Error>
where
    T: serde_crate::Serialize,
{
    value.serialize(crate::serde_serialize::JsonValueSerializer)
}

/// Deserializes a [`JsonValue`] into a typed value.
///
/// # Feature
///
/// Requires the `serde` feature flag.
#[cfg(feature = "serde")]
pub fn from_value<T>(value: JsonValue) -> Result<T, crate::serde_error::Error>
where
    T: serde_crate::de::DeserializeOwned,
{
    T::deserialize(crate::serde_deserialize::JsonValueDeserializer::new(value))
}

/// Parses a JSON string.
///
/// Without the `serde` feature, returns a [`JsonValue`].
/// With the `serde` feature, deserializes into any type implementing
/// [`DeserializeOwned`](serde_crate::de::DeserializeOwned).
///
/// # Example
///
/// ```
/// use lifegraph_json::{from_str, JsonValue};
///
/// let value: JsonValue = from_str(r#"{"name":"Alice"}"#)?;
/// assert_eq!(value["name"].as_str(), Some("Alice"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg(feature = "serde")]
pub fn from_str<T>(input: &str) -> Result<T, crate::serde_error::Error>
where
    T: serde_crate::de::DeserializeOwned,
{
    let mut de = crate::serde_deserialize::Deserializer::from_str(input);
    let value = T::deserialize(&mut de)?;
    de.end()?;
    Ok(value)
}

/// Parses a JSON string into a [`JsonValue`].
///
/// This is an alias for [`parse_json`] when the `serde` feature is not enabled.
#[cfg(not(feature = "serde"))]
pub fn from_str(input: &str) -> Result<JsonValue, JsonParseError> {
    parse_json(input)
}

/// Parses a JSON byte slice.
///
/// Without the `serde` feature, the input must be valid UTF-8 and returns a [`JsonValue`].
/// With the `serde` feature, deserializes into any type implementing
/// [`DeserializeOwned`](serde_crate::de::DeserializeOwned).
#[cfg(feature = "serde")]
pub fn from_slice<T>(input: &[u8]) -> Result<T, crate::serde_error::Error>
where
    T: serde_crate::de::DeserializeOwned,
{
    let mut de = crate::serde_deserialize::Deserializer::from_slice(input);
    let value = T::deserialize(&mut de)?;
    de.end()?;
    Ok(value)
}

/// Parses a JSON byte slice into a [`JsonValue`].
///
/// # Errors
///
/// Returns [`JsonParseError::InvalidUtf8`] if the input is not valid UTF-8.
#[cfg(not(feature = "serde"))]
pub fn from_slice(input: &[u8]) -> Result<JsonValue, JsonParseError> {
    let input = core::str::from_utf8(input).map_err(|_| JsonParseError::InvalidUtf8)?;
    parse_json(input)
}

/// Reads JSON from a reader and parses it.
///
/// Without the `serde` feature, returns a [`JsonValue`].
/// With the `serde` feature, deserializes into any type implementing
/// [`DeserializeOwned`](serde_crate::de::DeserializeOwned).
#[cfg(feature = "serde")]
pub fn from_reader<T, R>(reader: R) -> Result<T, crate::serde_error::Error>
where
    T: serde_crate::de::DeserializeOwned,
    R: Read,
{
    let mut de = crate::serde_deserialize::Deserializer::from_reader(reader);
    let value = T::deserialize(&mut de)?;
    de.end()?;
    Ok(value)
}

/// Reads JSON from a reader and parses it into a [`JsonValue`].
#[cfg(all(not(feature = "serde"), feature = "std"))]
pub fn from_reader<R: Read>(mut reader: R) -> Result<JsonValue, JsonParseError> {
    let mut input = String::new();
    reader
        .read_to_string(&mut input)
        .map_err(|_| JsonParseError::InvalidUtf8)?;
    parse_json(&input)
}

/// Serializes a value to a compact JSON string.
///
/// Without the `serde` feature, accepts a [`JsonValue`].
/// With the `serde` feature, accepts any type implementing [`Serialize`](serde_crate::Serialize).
///
/// # Example
///
/// ```
/// use lifegraph_json::{json, to_string};
///
/// let value = json!({"name": "Alice", "age": 30});
/// assert_eq!(to_string(&value)?, r#"{"name":"Alice","age":30}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg(feature = "serde")]
pub fn to_string<T>(value: &T) -> Result<String, crate::serde_error::Error>
where
    T: serde_crate::Serialize + ?Sized,
{
    crate::serde_streaming_serialize::to_string(value)
}

/// Serializes a [`JsonValue`] to a compact JSON string.
#[cfg(not(feature = "serde"))]
pub fn to_string(value: &JsonValue) -> Result<String, JsonError> {
    value.to_json_string()
}

/// Serializes a value to a JSON byte vector.
///
/// Without the `serde` feature, accepts a [`JsonValue`].
/// With the `serde` feature, accepts any type implementing [`Serialize`](serde_crate::Serialize).
#[cfg(feature = "serde")]
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, crate::serde_error::Error>
where
    T: serde_crate::Serialize + ?Sized,
{
    crate::serde_streaming_serialize::to_vec(value)
}

/// Serializes a [`JsonValue`] to a JSON byte vector.
#[cfg(not(feature = "serde"))]
pub fn to_vec(value: &JsonValue) -> Result<Vec<u8>, JsonError> {
    let mut out = Vec::with_capacity(util::initial_json_capacity(value));
    util::write_json_value(&mut out, value)?;
    Ok(out)
}

/// Writes JSON to a writer.
#[cfg(feature = "serde")]
pub fn to_writer<T, W>(mut writer: W, value: &T) -> Result<(), crate::serde_error::Error>
where
    T: serde_crate::Serialize + ?Sized,
    W: Write,
{
    let bytes = to_vec(value)?;
    writer.write_all(&bytes).map_err(|_| {
        crate::serde_error::Error::io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "write failed",
        ))
    })
}

/// Writes a [`JsonValue`] to a writer.
#[cfg(all(not(feature = "serde"), feature = "std"))]
pub fn to_writer<W: Write>(mut writer: W, value: &JsonValue) -> Result<(), JsonError> {
    let bytes = to_vec(value)?;
    writer.write_all(&bytes).map_err(|_| JsonError::Io)
}

/// Serializes a value to a pretty-printed JSON string.
#[cfg(feature = "serde")]
pub fn to_string_pretty<T>(value: &T) -> Result<String, crate::serde_error::Error>
where
    T: serde_crate::Serialize + ?Sized,
{
    let json_value = to_value(value)?;
    let mut out = Vec::with_capacity(util::initial_json_capacity(&json_value) + 16);
    util::write_json_value_pretty(&mut out, &json_value, 0)?;
    Ok(unsafe { String::from_utf8_unchecked(out) })
}

/// Serializes a [`JsonValue`] to a pretty-printed JSON string.
///
/// # Example
///
/// ```
/// use lifegraph_json::{json, to_string_pretty};
///
/// let value = json!({"name": "Alice", "age": 30});
/// let pretty = to_string_pretty(&value)?;
/// assert!(pretty.contains("\n"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg(not(feature = "serde"))]
pub fn to_string_pretty(value: &JsonValue) -> Result<String, JsonError> {
    let mut out = Vec::with_capacity(util::initial_json_capacity(value) + 16);
    util::write_json_value_pretty(&mut out, value, 0)?;
    Ok(unsafe { String::from_utf8_unchecked(out) })
}

/// Serializes a value to a pretty-printed JSON byte vector.
#[cfg(feature = "serde")]
pub fn to_vec_pretty<T>(value: &T) -> Result<Vec<u8>, crate::serde_error::Error>
where
    T: serde_crate::Serialize + ?Sized,
{
    let json_value = to_value(value)?;
    let mut out = Vec::with_capacity(util::initial_json_capacity(&json_value) + 16);
    util::write_json_value_pretty(&mut out, &json_value, 0)?;
    Ok(out)
}

/// Serializes a [`JsonValue`] to a pretty-printed JSON byte vector.
#[cfg(not(feature = "serde"))]
pub fn to_vec_pretty(value: &JsonValue) -> Result<Vec<u8>, JsonError> {
    let mut out = Vec::with_capacity(util::initial_json_capacity(value) + 16);
    util::write_json_value_pretty(&mut out, value, 0)?;
    Ok(out)
}

/// Writes pretty-printed JSON to a writer.
#[cfg(feature = "serde")]
pub fn to_writer_pretty<T, W>(mut writer: W, value: &T) -> Result<(), crate::serde_error::Error>
where
    T: serde_crate::Serialize + ?Sized,
    W: Write,
{
    let bytes = to_vec_pretty(value)?;
    writer.write_all(&bytes).map_err(|_| {
        crate::serde_error::Error::io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "write failed",
        ))
    })
}

#[cfg(all(not(feature = "serde"), feature = "std"))]
pub fn to_writer_pretty<W: Write>(mut writer: W, value: &JsonValue) -> Result<(), JsonError> {
    let bytes = to_vec_pretty(value)?;
    writer.write_all(&bytes).map_err(|_| JsonError::Io)
}
