#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

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

    // Find first escapable byte — scan 8 bytes at a time for ASCII text.
    let mut fast_index = 0usize;
    let len = bytes.len();
    while fast_index + 8 <= len {
        let chunk = u64::from_le_bytes(bytes[fast_index..fast_index + 8].try_into().unwrap());
        // Check for control chars (0x00..=0x1f), quote (0x22), backslash (0x5c)
        // A byte is escapable if it's <= 0x1f, == 0x22, or == 0x5c
        // has_zero checks for any byte with high bit clear after subtraction
        let has_ctrl = has_byte_le(chunk, 0x20); // anything < 0x20
        let eq_quote = has_byte_eq(chunk, b'"');
        let eq_backslash = has_byte_eq(chunk, b'\\');
        if has_ctrl | eq_quote | eq_backslash {
            break;
        }
        fast_index += 8;
    }
    // Finish byte-by-byte
    while fast_index < len {
        if needs_escape(bytes[fast_index]) {
            break;
        }
        fast_index += 1;
    }

    if fast_index == len {
        out.extend_from_slice(bytes);
        out.push(b'"');
        return;
    }
    if fast_index > 0 {
        out.extend_from_slice(&bytes[..fast_index]);
    }

    // Main escape loop: scan for next escapable byte, copy plain runs, emit escapes
    let mut pos = fast_index;
    let mut plain_start = fast_index;
    while pos < len {
        let byte = bytes[pos];
        if !needs_escape(byte) {
            pos += 1;
            continue;
        }
        // Copy plain bytes before the escape
        if pos > plain_start {
            out.extend_from_slice(&bytes[plain_start..pos]);
        }
        // Emit escape
        match byte {
            b'"' => out.extend_from_slice(br#"\""#),
            b'\\' => out.extend_from_slice(br"\\"),
            0x08 => out.extend_from_slice(br"\b"),
            0x0c => out.extend_from_slice(br"\f"),
            b'\n' => out.extend_from_slice(br"\n"),
            b'\r' => out.extend_from_slice(br"\r"),
            b'\t' => out.extend_from_slice(br"\t"),
            _ => {
                // Control char: \u00XX
                out.extend_from_slice(br"\u00");
                out.push(hex_digit((byte >> 4) & 0x0f));
                out.push(hex_digit(byte & 0x0f));
            }
        }
        pos += 1;
        plain_start = pos;
    }
    // Copy trailing plain bytes
    if plain_start < len {
        out.extend_from_slice(&bytes[plain_start..]);
    }
    out.push(b'"');
}

/// Check if any byte in a u64 is less than or equal to `limit`.
/// Uses SIMD-style SWAR trick: (x - 0x0101010101010101) & ~x & 0x8080808080808080
#[inline]
fn has_byte_le(x: u64, limit: u8) -> bool {
    // Check if any byte <= limit
    // For limit=0x1f: check if any byte < 0x20
    let lt = (x.wrapping_sub(0x0101010101010101 * u64::from(limit))) & !x & 0x8080808080808080;
    lt != 0
}

/// Check if any byte in a u64 equals `val`.
/// eq = (x ^ magic) - magic where magic replicates val
#[inline]
fn has_byte_eq(x: u64, val: u8) -> bool {
    let magic = 0x0101010101010101 * u64::from(val);
    let xor = x ^ magic;
    (xor.wrapping_sub(0x0101010101010101)) & !xor & 0x8080808080808080 != 0
}

pub fn needs_escape(byte: u8) -> bool {
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

fn hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        10..=15 => b'a' + (value - 10),
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Fast number → bytes conversion
// ---------------------------------------------------------------------------

/// Table-based u64 → ASCII.  Writes into `out` directly (no intermediate buf).
pub fn append_u64(out: &mut Vec<u8>, mut value: u64) {
    if value == 0 {
        out.push(b'0');
        return;
    }
    // Two digits per iteration using a pre-computed table.
    // We write into a local buffer then copy once at the end.
    let mut buf = [0u8; 20];
    let mut pos = buf.len();

    // Process 2 digits at a time for the bulk of the number.
    // For the last 1-2 digits (when value < 100) we handle separately.
    const TABLE: &[u8; 200] = b"00010203040506070809\
                               10111213141516171819\
                               20212223242526272829\
                               30313233343536373839\
                               40414243444546474849\
                               50515253545556575859\
                               60616263646566676869\
                               70717273747576777879\
                               80818283848586878889\
                               90919293949596979899";

    while value >= 100 {
        let d = (value % 100) as usize * 2;
        value /= 100;
        pos -= 2;
        buf[pos] = TABLE[d];
        buf[pos + 1] = TABLE[d + 1];
    }

    // Handle the remaining 1-2 digits.
    if value < 10 {
        pos -= 1;
        buf[pos] = b'0' + value as u8;
    } else {
        let d = (value as usize) * 2;
        pos -= 2;
        buf[pos] = TABLE[d];
        buf[pos + 1] = TABLE[d + 1];
    }

    out.extend_from_slice(&buf[pos..]);
}

/// i64 → ASCII.  Handles the sign then delegates to `append_u64`.
pub fn append_i64(out: &mut Vec<u8>, value: i64) {
    if value < 0 {
        out.push(b'-');
        // Use unsigned absolute value; for i64::MIN this wraps correctly
        // because we go through u64.
        append_u64(out, value.wrapping_abs() as u64);
    } else {
        append_u64(out, value as u64);
    }
}

/// u128 → ASCII using the same two-digits-at-a-time technique.
#[cfg(feature = "serde")]
pub fn append_u128(out: &mut Vec<u8>, mut value: u128) {
    if value == 0 {
        out.push(b'0');
        return;
    }
    let mut buf = [0u8; 40];
    let mut pos = buf.len();

    const TABLE: &[u8; 200] = b"00010203040506070809\
                               10111213141516171819\
                               20212223242526272829\
                               30313233343536373839\
                               40414243444546474849\
                               50515253545556575859\
                               60616263646566676869\
                               70717273747576777879\
                               80818283848586878889\
                               90919293949596979899";

    while value >= 100 {
        let d = (value % 100) as usize * 2;
        value /= 100;
        pos -= 2;
        buf[pos] = TABLE[d];
        buf[pos + 1] = TABLE[d + 1];
    }

    if value < 10 {
        pos -= 1;
        buf[pos] = b'0' + value as u8;
    } else {
        let d = (value as usize) * 2;
        pos -= 2;
        buf[pos] = TABLE[d];
        buf[pos + 1] = TABLE[d + 1];
    }

    out.extend_from_slice(&buf[pos..]);
}

/// i128 → ASCII.
#[cfg(feature = "serde")]
pub fn append_i128(out: &mut Vec<u8>, value: i128) {
    if value < 0 {
        out.push(b'-');
        append_u128(out, value.wrapping_abs() as u128);
    } else {
        append_u128(out, value as u128);
    }
}

/// f64 → ASCII.  Uses `ryu`-style formatting when available via
/// `core::fmt::Write`, but avoids the FmtWriter struct re-definition overhead
/// by using a small stack buffer and a single extend.
#[cfg(feature = "serde")]
pub fn append_f64(out: &mut Vec<u8>, value: f64) {
    // Special cases
    if value == 0.0 {
        if value.is_sign_negative() {
            out.extend_from_slice(b"-0.0");
        } else {
            out.extend_from_slice(b"0.0");
        }
        return;
    }
    // Fast path: use a stack buffer with core::fmt
    use core::fmt::Write;
    struct BufWriter<'a>(&'a mut [u8; 32], usize);
    impl Write for BufWriter<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            if self.1 + bytes.len() <= self.0.len() {
                self.0[self.1..self.1 + bytes.len()].copy_from_slice(bytes);
                self.1 += bytes.len();
                Ok(())
            } else {
                Err(core::fmt::Error)
            }
        }
    }
    let mut buf = [0u8; 32];
    let mut writer = BufWriter(&mut buf, 0);
    // Use the default Display formatting (shortest round-trip representation)
    if write!(writer, "{value:.17}").is_ok() {
        // Post-process: remove trailing zeros after decimal point
        let len = writer.1;
        let s = &buf[..len];
        // Check if there's a decimal point and strip trailing zeros
        if let Some(dot_pos) = s.iter().position(|&b| b == b'.') {
            let mut end = len;
            // Remove trailing zeros
            while end > dot_pos + 1 && s[end - 1] == b'0' {
                end -= 1;
            }
            // Don't remove the last digit after the dot if it's the only one
            if end == dot_pos + 1 {
                end = dot_pos; // remove the dot too
            }
            out.extend_from_slice(&s[..end]);
        } else {
            out.extend_from_slice(s);
        }
        return;
    }
    // Fallback: use a simpler approach
    // This shouldn't normally happen with the 32-byte buffer
    let mut buf2 = [0u8; 32];
    let len = {
        let mut fallback_writer = BufWriter(&mut buf2, 0);
        let _ = write!(fallback_writer, "{value}");
        fallback_writer.1
    };
    out.extend_from_slice(&buf2[..len]);
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
