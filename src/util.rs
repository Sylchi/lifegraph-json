use crate::error::JsonError;
use crate::value::{JsonNumber, JsonValue};
use std::io::Write;

pub fn write_json_value(out: &mut Vec<u8>, value: &JsonValue) -> Result<(), JsonError> {
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

pub fn write_json_value_pretty(
    out: &mut Vec<u8>,
    value: &JsonValue,
    depth: usize,
) -> Result<(), JsonError> {
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

fn write_indent(out: &mut Vec<u8>, depth: usize) {
    for _ in 0..depth {
        out.extend_from_slice(b"  ");
    }
}

pub fn write_json_number(out: &mut Vec<u8>, value: &JsonNumber) -> Result<(), JsonError> {
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
            use core::fmt::Write;
            struct FmtWriter<'a>(&'a mut Vec<u8>);
            impl<'a> Write for FmtWriter<'a> {
                fn write_str(&mut self, s: &str) -> core::fmt::Result {
                    self.0.extend_from_slice(s.as_bytes());
                    Ok(())
                }
            }
            let _ = write!(FmtWriter(out), "{value}");
            Ok(())
        }
    }
}

pub fn write_escaped_json_string(out: &mut Vec<u8>, input: &str) {
    out.push(b'"');
    let bytes = input.as_bytes();
    let mut chunk_start = 0usize;

    for (index, byte) in bytes.iter().copied().enumerate() {
        let escape = match byte {
            b'"' => Some(br#"\""#.as_slice()),
            b'\\' => Some(br#"\\"#.as_slice()),
            0x08 => Some(br#"\b"#.as_slice()),
            0x0c => Some(br#"\f"#.as_slice()),
            b'\n' => Some(br#"\n"#.as_slice()),
            b'\r' => Some(br#"\r"#.as_slice()),
            b'\t' => Some(br#"\t"#.as_slice()),
            _ if byte <= 0x1f => {
                if chunk_start < index {
                    out.extend_from_slice(&bytes[chunk_start..index]);
                }
                out.extend_from_slice(br#"\u00"#);
                out.push(hex_digit((byte >> 4) & 0x0f));
                out.push(hex_digit(byte & 0x0f));
                chunk_start = index + 1;
                continue;
            }
            _ => None,
        };

        if let Some(escape) = escape {
            if chunk_start < index {
                out.extend_from_slice(&bytes[chunk_start..index]);
            }
            out.extend_from_slice(escape);
            chunk_start = index + 1;
        }
    }

    if chunk_start < bytes.len() {
        out.extend_from_slice(&bytes[chunk_start..]);
    }
    out.push(b'"');
}

fn needs_escape(byte: u8) -> bool {
    matches!(byte, b'"' | b'\\' | 0x00..=0x1f)
}

pub fn write_json_array(out: &mut Vec<u8>, values: &[JsonValue]) -> Result<(), JsonError> {
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

pub fn write_json_object(out: &mut Vec<u8>, entries: &[(String, JsonValue)]) -> Result<(), JsonError> {
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

pub fn write_json_key(out: &mut Vec<u8>, key: &str) {
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

fn is_plain_json_string(bytes: &[u8]) -> bool {
    for &byte in bytes {
        if needs_escape(byte) {
            return false;
        }
    }
    true
}

pub fn initial_json_capacity(value: &JsonValue) -> usize {
    match value {
        JsonValue::Null => 4,
        JsonValue::Bool(true) => 4,
        JsonValue::Bool(false) => 5,
        JsonValue::Number(JsonNumber::I64(value)) => estimate_i64_len(*value),
        JsonValue::Number(JsonNumber::U64(value)) => estimate_u64_len(*value),
        JsonValue::Number(JsonNumber::F64(_)) => 24,
        JsonValue::String(value) => estimate_escaped_string_len(value),
        JsonValue::Array(values) => {
            if values.is_empty() {
                2
            } else {
                2 + values.len().saturating_sub(1)
                    + values
                        .iter()
                        .map(|v| initial_json_capacity(v))
                        .sum::<usize>()
            }
        }
        JsonValue::Object(entries) => {
            if entries.is_empty() {
                2
            } else {
                2 + entries.len()
                    + entries
                        .iter()
                        .map(|(key, value)| {
                            estimate_escaped_string_len(key) + 1 + initial_json_capacity(value)
                        })
                        .sum::<usize>()
            }
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
    if value < 10 {
        out.push(b'0' + value as u8);
        return;
    }
    if value < 100 {
        out.extend_from_slice(&[b'0' + (value / 10) as u8, b'0' + (value % 10) as u8]);
        return;
    }
    if value < 1000 {
        out.extend_from_slice(&[
            b'0' + (value / 100) as u8,
            b'0' + ((value / 10) % 10) as u8,
            b'0' + (value % 10) as u8,
        ]);
        return;
    }

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

pub fn hash_key(bytes: &[u8]) -> u64 {
    let mut hash = 1469598103934665603u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

pub fn decode_pointer_segment(segment: &str) -> String {
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
