use crate::{JsonError, JsonNumber, JsonValue};

pub fn write_json_value(out: &mut Vec<u8>, value: &JsonValue) -> Result<(), JsonError> {
    match value {
        JsonValue::Null => out.extend_from_slice(b"null"),
        JsonValue::Bool(v) => {
            if *v {
                out.extend_from_slice(b"true");
            } else {
                out.extend_from_slice(b"false");
            }
        }
        JsonValue::Number(n) => write_json_number(out, n)?,
        JsonValue::String(s) => write_escaped_json_string(out, s),
        JsonValue::Array(v) => write_json_array(out, v)?,
        JsonValue::Object(e) => write_json_object(out, e)?,
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
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        out.extend_from_slice(b",\n");
                    }
                    write_indent(out, depth + 1);
                    write_json_value_pretty(out, v, depth + 1)?;
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
                for (i, (key, value)) in entries.iter().enumerate() {
                    if i > 0 {
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
        JsonNumber::I64(v) => {
            append_i64(out, *v);
            Ok(())
        }
        JsonNumber::U64(v) => {
            append_u64(out, *v);
            Ok(())
        }
        JsonNumber::F64(v) => {
            if !v.is_finite() {
                return Err(JsonError::NonFiniteNumber);
            }
            use core::fmt::Write;
            struct FmtWriter<'a>(&'a mut Vec<u8>);
            impl Write for FmtWriter<'_> {
                fn write_str(&mut self, s: &str) -> core::fmt::Result {
                    self.0.extend_from_slice(s.as_bytes());
                    Ok(())
                }
            }
            let _ = write!(FmtWriter(out), "{v}");
            Ok(())
        }
    }
}

pub fn write_escaped_json_string(out: &mut Vec<u8>, input: &str) {
    out.push(b'"');
    let bytes = input.as_bytes();
    let mut fast_index = 0usize;
    while fast_index < bytes.len() {
        if needs_escape(bytes[fast_index]) {
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
            b'\\' => Some(br"\\".as_slice()),
            0x08 => Some(br"\b".as_slice()),
            0x0c => Some(br"\f".as_slice()),
            b'\n' => Some(br"\n".as_slice()),
            b'\r' => Some(br"\r".as_slice()),
            b'\t' => Some(br"\t".as_slice()),
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
            out.extend_from_slice(br"\u00");
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

fn needs_escape(byte: u8) -> bool {
    matches!(byte, b'"' | b'\\' | 0x00..=0x1f)
}

pub fn write_json_array(out: &mut Vec<u8>, values: &[JsonValue]) -> Result<(), JsonError> {
    out.push(b'[');
    match values {
        [] => {}
        [one] => write_json_value(out, one)?,
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
                for v in iter {
                    out.push(b',');
                    write_json_value(out, v)?;
                }
            }
        }
    }
    out.push(b']');
    Ok(())
}

pub fn write_json_object(
    out: &mut Vec<u8>,
    entries: &[(String, JsonValue)],
) -> Result<(), JsonError> {
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
    bytes.iter().all(|&b| !needs_escape(b))
}

pub fn initial_json_capacity(value: &JsonValue) -> usize {
    match value {
        JsonValue::Null => 4,
        JsonValue::Bool(true) => 4,
        JsonValue::Bool(false) => 5,
        JsonValue::Number(JsonNumber::I64(v)) => estimate_i64_len(*v),
        JsonValue::Number(JsonNumber::U64(v)) => estimate_u64_len(*v),
        JsonValue::Number(JsonNumber::F64(_)) => 24,
        JsonValue::String(v) => estimate_escaped_string_len(v),
        JsonValue::Array(v) => 2 + v.len().saturating_mul(16),
        JsonValue::Object(e) => {
            2 + e
                .iter()
                .map(|(k, _)| estimate_escaped_string_len(k) + 8)
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

pub fn hash_key(bytes: &[u8]) -> u64 {
    let mut hash = 1_469_598_103_934_665_603u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211u64);
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
