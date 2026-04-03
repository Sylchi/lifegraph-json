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

pub fn escape_json_string(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len() + 2);
    util::write_escaped_json_string(&mut out, input);
    unsafe { String::from_utf8_unchecked(out) }
}

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

#[cfg(feature = "serde")]
pub fn to_value<T>(value: T) -> Result<JsonValue, crate::serde_error::Error>
where
    T: serde_crate::Serialize,
{
    value.serialize(crate::serde_serialize::JsonValueSerializer)
}

#[cfg(feature = "serde")]
pub fn from_value<T>(value: JsonValue) -> Result<T, crate::serde_error::Error>
where
    T: serde_crate::de::DeserializeOwned,
{
    T::deserialize(crate::serde_deserialize::JsonValueDeserializer::new(value))
}

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

#[cfg(not(feature = "serde"))]
pub fn from_str(input: &str) -> Result<JsonValue, JsonParseError> {
    parse_json(input)
}

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

#[cfg(not(feature = "serde"))]
pub fn from_slice(input: &[u8]) -> Result<JsonValue, JsonParseError> {
    let input = core::str::from_utf8(input).map_err(|_| JsonParseError::InvalidUtf8)?;
    parse_json(input)
}

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

#[cfg(all(not(feature = "serde"), feature = "std"))]
pub fn from_reader<R: Read>(mut reader: R) -> Result<JsonValue, JsonParseError> {
    let mut input = String::new();
    reader
        .read_to_string(&mut input)
        .map_err(|_| JsonParseError::InvalidUtf8)?;
    parse_json(&input)
}

#[cfg(feature = "serde")]
pub fn to_string<T>(value: &T) -> Result<String, crate::serde_error::Error>
where
    T: serde_crate::Serialize + ?Sized,
{
    let json_value = to_value(value)?;
    json_value
        .to_json_string()
        .map_err(crate::serde_error::json_error_to_serde)
}

#[cfg(not(feature = "serde"))]
pub fn to_string(value: &JsonValue) -> Result<String, JsonError> {
    value.to_json_string()
}

#[cfg(feature = "serde")]
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, crate::serde_error::Error>
where
    T: serde_crate::Serialize + ?Sized,
{
    let json_value = to_value(value)?;
    let mut out = Vec::with_capacity(util::initial_json_capacity(&json_value));
    util::write_json_value(&mut out, &json_value)?;
    Ok(out)
}

#[cfg(not(feature = "serde"))]
pub fn to_vec(value: &JsonValue) -> Result<Vec<u8>, JsonError> {
    let mut out = Vec::with_capacity(util::initial_json_capacity(value));
    util::write_json_value(&mut out, value)?;
    Ok(out)
}

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

#[cfg(all(not(feature = "serde"), feature = "std"))]
pub fn to_writer<W: Write>(mut writer: W, value: &JsonValue) -> Result<(), JsonError> {
    let bytes = to_vec(value)?;
    writer.write_all(&bytes).map_err(|_| JsonError::Io)
}

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

#[cfg(not(feature = "serde"))]
pub fn to_string_pretty(value: &JsonValue) -> Result<String, JsonError> {
    let mut out = Vec::with_capacity(util::initial_json_capacity(value) + 16);
    util::write_json_value_pretty(&mut out, value, 0)?;
    Ok(unsafe { String::from_utf8_unchecked(out) })
}

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

#[cfg(not(feature = "serde"))]
pub fn to_vec_pretty(value: &JsonValue) -> Result<Vec<u8>, JsonError> {
    let mut out = Vec::with_capacity(util::initial_json_capacity(value) + 16);
    util::write_json_value_pretty(&mut out, value, 0)?;
    Ok(out)
}

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
