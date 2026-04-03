//! Unicode correctness tests for lifegraph-json
//!
//! Run with: cargo test --test unicode -- --nocapture

#[cfg(feature = "serde")]
use lifegraph_json::to_string;
use lifegraph_json::{parse_json, JsonValue};

/// Test that all valid Unicode scalar values are handled correctly
#[test]
fn test_unicode_scalar_values() {
    let test_chars = vec![
        ('\u{0020}', "space"),
        ('\u{007F}', "DEL"),
        ('\u{00A0}', "non-breaking space"),
        ('\u{4E00}', "CJK unified ideograph"),
        ('\u{D7FF}', "last before high surrogates"),
        ('\u{10000}', "linear B syllable"),
        ('\u{1F600}', "grinning face emoji"),
        ('\u{10FFFF}', "max valid scalar"),
    ];

    for (ch, description) in test_chars {
        let json_str = format!("\"{ch}\"");
        let result = parse_json(&json_str);

        assert!(
            result.is_ok(),
            "Failed to parse valid Unicode scalar U+{:04X} ({}): {:?}",
            ch as u32,
            description,
            result.err()
        );

        let value = result.unwrap();
        if let JsonValue::String(s) = &value {
            assert_eq!(s.chars().next(), Some(ch));
        }
    }
}

/// Test surrogate pair handling
#[test]
fn test_surrogate_pairs() {
    // Valid surrogate pairs - use actual emoji characters
    let valid_pairs = vec![
        "😀", // grinning face
        "👍", // thumbs up
        "🎉", // party popper
    ];

    for pair in valid_pairs {
        let json_str = format!("\"{pair}\"");
        let result = parse_json(&json_str);

        assert!(
            result.is_ok(),
            "Failed to parse valid surrogate pair: {:?}",
            result.err()
        );

        let value = result.unwrap();
        if let JsonValue::String(s) = &value {
            let chars: Vec<char> = s.chars().collect();
            assert_eq!(chars.len(), 1, "Should decode to 1 char");
        }
    }

    // Invalid surrogates (should be rejected)
    let invalid = vec![
        "\\uD800",  // lone high surrogate
        "\\uDC00",  // lone low surrogate
        "\\uD800X", // high surrogate followed by non-surrogate
    ];

    for escape in invalid {
        let json_str = format!("\"{escape}\"");
        let result = parse_json(&json_str);
        assert!(result.is_err(), "Should reject invalid surrogate: {escape}");
    }
}

/// Test Unicode escape sequences
#[test]
fn test_unicode_escapes() {
    let test_cases = vec![
        ("\\u0041", "A"),
        ("\\u0000", "\0"),
        ("\\u007F", "\x7F"),
        ("\\u0080", "\u{0080}"),
        ("\\uFFFF", "\u{FFFF}"),
    ];

    for (escape, expected) in test_cases {
        let json_str = format!("\"{escape}\"");
        let result = parse_json(&json_str);

        assert!(
            result.is_ok(),
            "Failed to parse escape {}: {:?}",
            escape,
            result.err()
        );

        let value = result.unwrap();
        if let JsonValue::String(s) = &value {
            assert_eq!(s, expected);
        }
    }
}

/// Test standard JSON escape sequences
#[test]
fn test_standard_escapes() {
    let escapes = vec![
        ("\\\"", "\""),
        ("\\\\", "\\"),
        ("\\/", "/"),
        ("\\n", "\n"),
        ("\\r", "\r"),
        ("\\t", "\t"),
    ];

    for (escape, expected) in escapes {
        let json_str = format!("\"{escape}\"");
        let result = parse_json(&json_str);

        assert!(result.is_ok(), "Failed to parse escape {escape}");

        let value = result.unwrap();
        if let JsonValue::String(s) = &value {
            assert_eq!(s, expected, "Escape mismatch for {escape}");
        }
    }
}

/// Test invalid escape sequences
#[test]
fn test_invalid_escapes() {
    let invalid = vec![
        "\\a", "\\e", "\\v", "\\x41", "\\cC", "\\u", "\\u0", "\\u00", "\\u000", "\\uGGGG",
        "\\uXXXX",
    ];

    for escape in invalid {
        let json_str = format!("\"{escape}\"");
        let result = parse_json(&json_str);
        assert!(result.is_err(), "Should reject invalid escape: {escape}");
    }
}

/// Test UTF-8 encoding in raw form
#[test]
fn test_raw_utf8() {
    let test_cases = vec!["Hello", "Café", "日本語", "한국어", "🎉🎊🎈"];

    for text in test_cases {
        let json_str = format!("\"{text}\"");
        let result = parse_json(&json_str);

        assert!(
            result.is_ok(),
            "Failed to parse raw UTF-8: {}",
            result.err().unwrap()
        );

        let value = result.unwrap();
        if let JsonValue::String(s) = &value {
            assert_eq!(s, text);
        }
    }
}

/// Test round-trip Unicode preservation
#[cfg(feature = "serde")]
#[test]
fn test_unicode_roundtrip() {
    let test_strings = vec!["Hello, World!", "こんにちは世界", "🌍🌎🌏", "Привет мир"];

    for original in test_strings {
        let value = JsonValue::String(original.to_string());
        let serialized = to_string(&value).expect("Failed to serialize");
        let deserialized: JsonValue =
            lifegraph_json::from_str(&serialized).expect("Failed to deserialize");

        if let JsonValue::String(s) = &deserialized {
            assert_eq!(s, original, "Round-trip failed for {original:?}");
        } else {
            panic!("Deserialized to non-string: {deserialized:?}");
        }
    }
}

/// Test Unicode in object keys
#[cfg(feature = "serde")]
#[test]
fn test_unicode_keys() {
    let json = r#"{"日本語": "value", "🔑": "key", "clé": "french"}"#;
    let result: JsonValue = lifegraph_json::from_str(json).expect("Failed to parse unicode keys");

    if let JsonValue::Object(map) = &result {
        assert!(map.contains_key("日本語"));
        assert!(map.contains_key("🔑"));
        assert!(map.contains_key("clé"));
    } else {
        panic!("Expected object, got {result:?}");
    }
}
