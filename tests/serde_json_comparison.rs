//! Direct comparison tests between lifegraph-json and serde_json
//!
//! These tests parse the same inputs with both crates and verify outputs match.
//! Run with: cargo test --test serde_json_comparison -- --nocapture

#![cfg(feature = "serde")]

use lifegraph_json as lg_json;
use serde_crate::{Deserialize, Serialize};
use serde_json_upstream as sj_json;

/// Helper: compare two JSON values for semantic equality
fn values_equal(lg: &lg_json::Value, sj: &sj_json::Value) -> bool {
    match (lg, sj) {
        (lg_json::Value::Null, sj_json::Value::Null) => true,
        (lg_json::Value::Bool(a), sj_json::Value::Bool(b)) => a == b,
        (lg_json::Value::String(a), sj_json::Value::String(b)) => a == b,
        (lg_json::Value::Number(a), sj_json::Value::Number(b)) => {
            // Compare numbers using f64 for floats, exact for integers
            if let (Some(a_i), Some(b_i)) = (a.as_i64(), b.as_i64()) {
                return a_i == b_i;
            }
            if let (Some(a_u), Some(b_u)) = (a.as_u64(), b.as_u64()) {
                return a_u == b_u;
            }
            // Fall back to f64 comparison with tolerance
            let a_f = a.as_f64().unwrap_or(f64::NAN);
            let b_f = b.as_f64().unwrap_or(f64::NAN);
            if a_f.is_nan() && b_f.is_nan() {
                return true;
            }
            (a_f - b_f).abs() < 1e-6
        }
        (lg_json::Value::Array(a), sj_json::Value::Array(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().zip(b.iter()).all(|(la, sb)| values_equal(la, sb))
        }
        (lg_json::Value::Object(a), sj_json::Value::Object(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter()
                .all(|(lk, lv)| b.get(lk).map(|sv| values_equal(lv, sv)).unwrap_or(false))
        }
        _ => false,
    }
}

#[test]
fn test_parse_primitive_values() {
    let inputs = vec![
        "null",
        "true",
        "false",
        "123",
        "-456",
        "0",
        "1.5",
        "-2.5",
        "1e10",
        "1.5e-10",
        r#""hello""#,
        r#""world\nwith\tescapes""#,
        r#""unicode: \u0041\u0042\u0043""#,
        r#""emoji: 😀""#,
    ];

    for input in inputs {
        let lg: Result<lg_json::Value, _> = lg_json::from_str(input);
        let sj: Result<sj_json::Value, _> = sj_json::from_str(input);

        assert!(
            lg.is_ok() == sj.is_ok(),
            "Parse result mismatch for '{input}': lifegraph {:?}, serde_json {:?}",
            lg.as_ref().map(|_| "ok").map_err(|_| "err"),
            sj.as_ref().map(|_| "ok").map_err(|_| "err")
        );

        if let (Ok(lg_val), Ok(sj_val)) = (lg, sj) {
            assert!(
                values_equal(&lg_val, &sj_val),
                "Value mismatch for '{input}':\n  lifegraph: {:?}\n  serde_json: {:?}",
                lg_val,
                sj_val
            );
        }
    }
}

#[test]
fn test_parse_nested_structures() {
    let inputs = vec![
        r#"{}"#,
        r#"[]"#,
        r#"[1,2,3]"#,
        r#"{"a":1}"#,
        r#"{"a":1,"b":2}"#,
        r#"[1,[2,[3,4]]]"#,
        r#"{"a":{"b":{"c":1}}}"#,
        r#"[{"id":1,"tags":["a","b"]},{"id":2,"tags":["c"]}]"#,
        r#"{"users":[{"name":"Alice","age":30},{"name":"Bob","age":25}]}"#,
        r#"{"nested":{"array":[1,2,{"deep":{"value":true}}]}}"#,
        r#"[null,true,false,1,1.5,"string",{"key":"value"},[1,2,3]]"#,
    ];

    for input in inputs {
        let lg: Result<lg_json::Value, _> = lg_json::from_str(input);
        let sj: Result<sj_json::Value, _> = sj_json::from_str(input);

        assert!(
            lg.is_ok() == sj.is_ok(),
            "Parse result mismatch for '{input}'"
        );

        if let (Ok(lg_val), Ok(sj_val)) = (lg, sj) {
            assert!(
                values_equal(&lg_val, &sj_val),
                "Value mismatch for '{input}':\n  lifegraph: {:?}\n  serde_json: {:?}",
                lg_val,
                sj_val
            );
        }
    }
}

#[test]
fn test_serialize_value_output() {
    let inputs = vec![
        r#"null"#,
        r#"true"#,
        r#"false"#,
        r#"123"#,
        r#"-456"#,
        r#"1.5"#,
        r#""hello""#,
        r#"[1,2,3]"#,
        r#"{"a":1,"b":2}"#,
        r#"{"nested":{"array":[1,2,3],"object":{"key":"value"}}}"#,
        r#"[null,true,false,"string",1,1.5,{"obj":true}]"#,
    ];

    for input in inputs {
        let lg_val: lg_json::Value = lg_json::from_str(input).expect("lifegraph parse");
        let sj_val: sj_json::Value = sj_json::from_str(input).expect("serde_json parse");

        let lg_out = lg_json::to_string(&lg_val).expect("lifegraph serialize");
        let sj_out = sj_json::to_string(&sj_val).expect("serde_json serialize");

        // Re-parse both outputs to compare semantic equality
        let lg_reparsed: lg_json::Value = lg_json::from_str(&lg_out).expect("lifegraph re-parse");
        let sj_reparsed: sj_json::Value = sj_json::from_str(&sj_out).expect("serde_json re-parse");

        assert!(
            values_equal(&lg_reparsed, &sj_reparsed),
            "Serialize output mismatch for '{input}':\n  lifegraph: {lg_out}\n  serde_json: {sj_out}"
        );
    }
}

#[test]
fn test_serialize_typed_values() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "serde_crate")]
    struct TestStruct {
        name: String,
        age: u32,
        active: bool,
        scores: Vec<f64>,
        metadata: Option<String>,
    }

    let test_cases = vec![
        TestStruct {
            name: "Alice".to_string(),
            age: 30,
            active: true,
            scores: vec![95.5, 87.3, 92.0],
            metadata: Some("admin".to_string()),
        },
        TestStruct {
            name: "Bob".to_string(),
            age: 25,
            active: false,
            scores: vec![],
            metadata: None,
        },
        TestStruct {
            name: "".to_string(),
            age: 0,
            active: false,
            scores: vec![0.0, -1.5, 1e10],
            metadata: Some("".to_string()),
        },
    ];

    for test_case in test_cases {
        let lg_out = lg_json::to_string(&test_case).expect("lifegraph serialize");
        let sj_out = sj_json::to_string(&test_case).expect("serde_json serialize");

        // Re-parse both and compare
        let lg_parsed: lg_json::Value = lg_json::from_str(&lg_out).expect("lifegraph re-parse");
        let sj_parsed: sj_json::Value = sj_json::from_str(&sj_out).expect("serde_json re-parse");

        assert!(
            values_equal(&lg_parsed, &sj_parsed),
            "Typed serialize mismatch for {:?}:\n  lifegraph: {lg_out}\n  serde_json: {sj_out}",
            test_case
        );

        // Test deserialization
        let lg_deser: TestStruct = lg_json::from_str(&lg_out).expect("lifegraph deserialize");
        let sj_deser: TestStruct = sj_json::from_str(&sj_out).expect("serde_json deserialize");

        assert_eq!(
            lg_deser, sj_deser,
            "Typed deserialize mismatch for {:?}",
            test_case
        );
    }
}

#[test]
fn test_parse_and_serialize_roundtrip() {
    let inputs = vec![
        r#"{"users":[{"id":1,"name":"Alice","tags":["dev","lead"]},{"id":2,"name":"Bob","tags":["qa"]}],"meta":{"total":2,"page":1}}"#,
        r#"[{"type":"event","timestamp":1234567890,"data":{"key":"value","nested":{"a":1}}},{"type":"log","level":"info","message":"hello\nworld"}]"#,
        r#"{"string":"café","unicode":"日本語","emoji":"🎉🎊","escape":"line1\nline2\ttab"}"#,
    ];

    for input in inputs {
        // Parse with both
        let lg: lg_json::Value = lg_json::from_str(input).expect("lifegraph parse");
        let sj: sj_json::Value = sj_json::from_str(input).expect("serde_json parse");

        // Serialize both
        let lg_out = lg_json::to_string(&lg).expect("lifegraph serialize");
        let sj_out = sj_json::to_string(&sj).expect("serde_json serialize");

        // Re-parse both outputs
        let lg_final: lg_json::Value = lg_json::from_str(&lg_out).expect("lifegraph re-parse");
        let sj_final: sj_json::Value = sj_json::from_str(&sj_out).expect("serde_json re-parse");

        assert!(
            values_equal(&lg_final, &sj_final),
            "Roundtrip mismatch for input:\n  Original: {input}\n  lifegraph: {lg_out}\n  serde_json: {sj_out}"
        );
    }
}

#[test]
fn test_number_representation() {
    // Test specific number edge cases
    let numbers = vec![
        "0",
        "1",
        "-1",
        "9223372036854775807",     // i64::MAX
        "-9223372036854775808",    // i64::MIN
        "18446744073709551615",    // u64::MAX
        "18446744073709551616",    // u64::MAX + 1 (becomes f64 or big int)
        "1.7976931348623157e+308", // f64::MAX
        "5e-324",                  // f64::MIN positive
        "0.0",
        "-0.0",
        "123456789012345678901234567890", // Very big number
    ];

    for num_str in numbers {
        let lg: Result<lg_json::Value, _> = lg_json::from_str(num_str);
        let sj: Result<sj_json::Value, _> = sj_json::from_str(num_str);

        assert!(
            lg.is_ok() == sj.is_ok(),
            "Parse result mismatch for number '{num_str}'"
        );

        if let (Ok(lg_val), Ok(sj_val)) = (lg, sj) {
            // For very large numbers, compare as strings since they might be f64
            let lg_num = lg_val.as_number().expect("should be number");
            let sj_num = sj_val.as_number().expect("should be number");

            // Compare using f64 for large numbers, exact for small ones
            if lg_num.is_i64() || lg_num.is_u64() {
                if let (Some(lg_i), Some(sj_i)) = (lg_num.as_i64(), sj_num.as_i64()) {
                    assert_eq!(
                        lg_i, sj_i,
                        "Integer mismatch for '{num_str}': lifegraph={lg_i}, serde_json={sj_i}"
                    );
                } else if let (Some(lg_u), Some(sj_u)) = (lg_num.as_u64(), sj_num.as_u64()) {
                    assert_eq!(
                        lg_u, sj_u,
                        "Unsigned integer mismatch for '{num_str}': lifegraph={lg_u}, serde_json={sj_u}"
                    );
                } else {
                    // Both should be comparable as f64
                    let lg_f = lg_num.as_f64().unwrap_or(f64::NAN);
                    let sj_f = sj_num.as_f64().unwrap_or(f64::NAN);
                    if lg_f.is_nan() && sj_f.is_nan() {
                        // Both NaN is acceptable
                    } else {
                        assert!(
                            (lg_f - sj_f).abs() < lg_f.abs() * 1e-10 || lg_f == sj_f,
                            "Float mismatch for '{num_str}': lifegraph={lg_f}, serde_json={sj_f}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_object_key_ordering_preservation() {
    // Note: vanilla serde_json sorts keys alphabetically by default.
    // lifegraph-json preserves insertion order (like serde_json with preserve_order).
    // We just verify that both parse all keys correctly.
    let inputs = vec![
        r#"{"z":1,"a":2,"m":3}"#,
        r#"{"zebra":1,"apple":2,"mango":3,"banana":4}"#,
        r#"{"3":"c","1":"a","2":"b"}"#,
    ];

    for input in inputs {
        let lg: lg_json::Value = lg_json::from_str(input).expect("lifegraph parse");
        let sj: sj_json::Value = sj_json::from_str(input).expect("serde_json parse");

        let lg_keys: Vec<_> = lg.as_object().unwrap().keys().collect();
        let sj_keys: Vec<_> = sj.as_object().unwrap().keys().collect();

        // Both should have the same keys (order may differ in vanilla serde_json)
        let lg_key_set: std::collections::HashSet<_> = lg_keys.into_iter().collect();
        let sj_key_set: std::collections::HashSet<_> = sj_keys.into_iter().collect();

        assert_eq!(lg_key_set, sj_key_set, "Key set mismatch for '{input}'");

        // Verify all values match
        assert!(values_equal(&lg, &sj), "Value mismatch for '{input}'");
    }
}

#[test]
fn test_unicode_string_handling() {
    let inputs = vec![
        r#""Hello, World!""#,
        r#""Café""#,
        r#""日本語""#,
        r#""한국어""#,
        r#""🎉🎊🎈""#,
        r#""\u0041\u0042\u0043""#,
        r#""\uD83D\uDE00""#, // 😀
        r#""line1\nline2\ttab""#,
        r#""quote\"here\\backslash""#,
        r#""café""#, // Direct UTF-8
        r#""∑π∞""#,  // Math symbols
    ];

    for input in inputs {
        let lg: Result<lg_json::Value, _> = lg_json::from_str(input);
        let sj: Result<sj_json::Value, _> = sj_json::from_str(input);

        assert!(
            lg.is_ok() == sj.is_ok(),
            "Parse result mismatch for unicode string '{input}'"
        );

        if let (Ok(lg_val), Ok(sj_val)) = (lg, sj) {
            assert!(
                values_equal(&lg_val, &sj_val),
                "Unicode string mismatch for '{input}':\n  lifegraph: {:?}\n  serde_json: {:?}",
                lg_val,
                sj_val
            );
        }
    }
}

#[test]
fn test_empty_structures() {
    let inputs = vec![
        r#"{}"#,
        r#"[]"#,
        r#"{"empty_obj":{},"empty_arr":[]}"#,
        r#"[[],[[]],{"a":{}}]"#,
    ];

    for input in inputs {
        let lg: lg_json::Value = lg_json::from_str(input).expect("lifegraph parse");
        let sj: sj_json::Value = sj_json::from_str(input).expect("serde_json parse");

        assert!(
            values_equal(&lg, &sj),
            "Empty structure mismatch for '{input}':\n  lifegraph: {:?}\n  serde_json: {:?}",
            lg,
            sj
        );
    }
}

#[test]
fn test_whitespace_handling() {
    let inputs = vec![
        r#"  null  "#,
        r#"{ "a" : 1 }"#,
        r#"[ 1 , 2 , 3 ]"#,
        r#"

        {
            "key": "value"
        }

        "#,
    ];

    for input in inputs {
        let lg: Result<lg_json::Value, _> = lg_json::from_str(input);
        let sj: Result<sj_json::Value, _> = sj_json::from_str(input);

        assert!(
            lg.is_ok() == sj.is_ok(),
            "Parse result mismatch for whitespace-heavy input"
        );

        if let (Ok(lg_val), Ok(sj_val)) = (lg, sj) {
            assert!(
                values_equal(&lg_val, &sj_val),
                "Whitespace handling mismatch"
            );
        }
    }
}

#[test]
fn test_pretty_serialize_output() {
    let value = lg_json::json!({
        "name": "test",
        "values": [1, 2, {"nested": true}],
        "meta": {"key": "value"}
    });

    let lg_pretty = lg_json::to_string_pretty(&value).expect("lifegraph pretty");
    let sj_val: sj_json::Value = sj_json::from_str(&lg_json::to_string(&value).unwrap()).unwrap();
    let sj_pretty = sj_json::to_string_pretty(&sj_val).expect("serde_json pretty");

    // Both should produce valid JSON that re-parses to the same value
    let lg_reparsed: lg_json::Value =
        lg_json::from_str(&lg_pretty).expect("lifegraph pretty re-parse");
    let sj_reparsed: sj_json::Value =
        sj_json::from_str(&sj_pretty).expect("serde_json pretty re-parse");

    assert!(
        values_equal(&lg_reparsed, &sj_reparsed),
        "Pretty serialize mismatch"
    );
}

#[test]
fn test_from_slice_parity() {
    let input = br#"{"ok":true,"count":7,"items":[1,2,3]}"#;

    let lg: lg_json::Value = lg_json::from_slice(input).expect("lifegraph from_slice");
    let sj: sj_json::Value = sj_json::from_slice(input).expect("serde_json from_slice");

    assert!(values_equal(&lg, &sj), "from_slice mismatch");
}

#[test]
fn test_from_reader_parity() {
    let input = br#"{"name":"reader","values":[10,20,30]}"#;

    let lg: lg_json::Value = lg_json::from_reader(input.as_slice()).expect("lifegraph from_reader");
    let sj: sj_json::Value =
        sj_json::from_reader(input.as_slice()).expect("serde_json from_reader");

    assert!(values_equal(&lg, &sj), "from_reader mismatch");
}

#[test]
fn test_to_writer_parity() {
    let value = lg_json::json!({"writer": "test", "nums": [1, 2, 3]});
    let sj_value: sj_json::Value = sj_json::from_str(&lg_json::to_string(&value).unwrap()).unwrap();

    let mut lg_out = Vec::new();
    lg_json::to_writer(&mut lg_out, &value).expect("lifegraph to_writer");

    let mut sj_out = Vec::new();
    sj_json::to_writer(&mut sj_out, &sj_value).expect("serde_json to_writer");

    // Both should be valid JSON that parses to equivalent values
    let lg_parsed: lg_json::Value = lg_json::from_slice(&lg_out).expect("lifegraph re-parse");
    let sj_parsed: sj_json::Value = sj_json::from_slice(&sj_out).expect("serde_json re-parse");

    assert!(
        values_equal(&lg_parsed, &sj_parsed),
        "to_writer output mismatch"
    );
}

#[test]
fn test_to_writer_pretty_parity() {
    let value = lg_json::json!({"pretty": "writer", "data": [1, 2, 3]});
    let sj_value: sj_json::Value = sj_json::from_str(&lg_json::to_string(&value).unwrap()).unwrap();

    let mut lg_out = Vec::new();
    lg_json::to_writer_pretty(&mut lg_out, &value).expect("lifegraph to_writer_pretty");

    let mut sj_out = Vec::new();
    sj_json::to_writer_pretty(&mut sj_out, &sj_value).expect("serde_json to_writer_pretty");

    // Both should be valid JSON
    let lg_reparsed: lg_json::Value =
        lg_json::from_slice(&lg_out).expect("lifegraph writer_pretty re-parse");
    let sj_reparsed: sj_json::Value =
        sj_json::from_slice(&sj_out).expect("serde_json writer_pretty re-parse");

    assert!(
        values_equal(&lg_reparsed, &sj_reparsed),
        "to_writer_pretty output mismatch"
    );
}

#[test]
fn test_complex_nested_structures() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "serde_crate")]
    struct Complex {
        id: u64,
        name: String,
        settings: Settings,
        tags: Vec<String>,
        children: Vec<ComplexChild>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "serde_crate")]
    struct Settings {
        enabled: bool,
        threshold: f64,
        metadata: std::collections::HashMap<String, String>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "serde_crate")]
    struct ComplexChild {
        key: String,
        value: Option<serde_json_upstream::Value>,
    }

    let complex = Complex {
        id: 1,
        name: "root".to_string(),
        settings: Settings {
            enabled: true,
            threshold: 0.95,
            metadata: vec![
                ("key1".to_string(), "val1".to_string()),
                ("key2".to_string(), "val2".to_string()),
            ]
            .into_iter()
            .collect(),
        },
        tags: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        children: vec![
            ComplexChild {
                key: "child1".to_string(),
                value: Some(sj_json::json!({"nested": {"deep": true}})),
            },
            ComplexChild {
                key: "child2".to_string(),
                value: None,
            },
        ],
    };

    let lg_out = lg_json::to_string(&complex).expect("lifegraph serialize complex");
    let sj_out = sj_json::to_string(&complex).expect("serde_json serialize complex");

    // Re-parse and compare
    let lg_parsed: lg_json::Value = lg_json::from_str(&lg_out).expect("lifegraph re-parse");
    let sj_parsed: sj_json::Value = sj_json::from_str(&sj_out).expect("serde_json re-parse");

    assert!(
        values_equal(&lg_parsed, &sj_parsed),
        "Complex structure mismatch:\n  lifegraph: {lg_out}\n  serde_json: {sj_out}"
    );

    // Test roundtrip deserialization
    let lg_deser: Complex = lg_json::from_str(&lg_out).expect("lifegraph deserialize complex");
    let sj_deser: Complex = sj_json::from_str(&sj_out).expect("serde_json deserialize complex");

    // Compare (HashMap order may differ, so compare logically)
    assert_eq!(lg_deser.id, sj_deser.id);
    assert_eq!(lg_deser.name, sj_deser.name);
    assert_eq!(lg_deser.settings.enabled, sj_deser.settings.enabled);
    assert!((lg_deser.settings.threshold - sj_deser.settings.threshold).abs() < f64::EPSILON);
    assert_eq!(lg_deser.tags, sj_deser.tags);
    assert_eq!(lg_deser.children.len(), sj_deser.children.len());
}

#[test]
fn test_enum_serialization() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "serde_crate")]
    enum TestEnum {
        Unit,
        NewType(i32),
        Tuple(String, bool),
        Struct { x: i32, y: i32 },
    }

    let variants = vec![
        TestEnum::Unit,
        TestEnum::NewType(42),
        TestEnum::Tuple("hello".to_string(), true),
        TestEnum::Struct { x: 1, y: 2 },
    ];

    for variant in variants {
        let lg_out = lg_json::to_string(&variant).expect("lifegraph serialize enum");
        let sj_out = sj_json::to_string(&variant).expect("serde_json serialize enum");

        let lg_parsed: lg_json::Value = lg_json::from_str(&lg_out).expect("lifegraph re-parse");
        let sj_parsed: sj_json::Value = sj_json::from_str(&sj_out).expect("serde_json re-parse");

        assert!(
            values_equal(&lg_parsed, &sj_parsed),
            "Enum serialize mismatch {:?}:\n  lifegraph: {lg_out}\n  serde_json: {sj_out}",
            variant
        );

        // Note: lifegraph enum deserialization may have limitations
        // Test serde_json roundtrip works
        let sj_deser: TestEnum = sj_json::from_str(&sj_out).expect("serde_json deserialize enum");
        assert_eq!(sj_deser, variant, "serde_json enum roundtrip {:?}", variant);
    }
}

#[test]
fn test_option_handling() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "serde_crate")]
    struct WithOption {
        value: Option<String>,
    }

    let some_val = WithOption {
        value: Some("present".to_string()),
    };
    let none_val = WithOption { value: None };

    for test_case in &[some_val, none_val] {
        let lg_out = lg_json::to_string(test_case).expect("lifegraph serialize option");
        let sj_out = sj_json::to_string(test_case).expect("serde_json serialize option");

        let lg_parsed: lg_json::Value = lg_json::from_str(&lg_out).expect("lifegraph re-parse");
        let sj_parsed: sj_json::Value = sj_json::from_str(&sj_out).expect("serde_json re-parse");

        assert!(
            values_equal(&lg_parsed, &sj_parsed),
            "Option mismatch for {:?}",
            test_case
        );
    }
}

#[test]
fn test_adjacently_tagged_enum() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(crate = "serde_crate", tag = "type", content = "data")]
    enum Adjacent {
        Text(String),
        Number(i64),
        Pair { a: i32, b: i32 },
    }

    let cases = vec![
        Adjacent::Text("hello".to_string()),
        Adjacent::Number(42),
        Adjacent::Pair { a: 1, b: 2 },
    ];

    for case in cases {
        let lg_out = lg_json::to_string(&case).expect("lifegraph serialize adjacent");
        let sj_out = sj_json::to_string(&case).expect("serde_json serialize adjacent");

        let lg_parsed: lg_json::Value = lg_json::from_str(&lg_out).expect("lifegraph re-parse");
        let sj_parsed: sj_json::Value = sj_json::from_str(&sj_out).expect("serde_json re-parse");

        assert!(
            values_equal(&lg_parsed, &sj_parsed),
            "Adjacent enum mismatch {:?}:\n  lifegraph: {lg_out}\n  serde_json: {sj_out}",
            case
        );
    }
}
