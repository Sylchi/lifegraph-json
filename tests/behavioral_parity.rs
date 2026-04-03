#![cfg(all(feature = "serde", feature = "raw_value"))]

use lifegraph_json as serde_json;
use serde_crate::Serialize;
use serde_json::{from_str, to_raw_value, to_string, Map, RawValue, Value};

#[test]
fn raw_value_top_level_boxed_deserialization_works() {
    let input = r#"[null,{"a":1},"two"]"#;

    let raw: Box<RawValue> = from_str(input).unwrap();
    assert_eq!(raw.get(), input);
}

#[test]
fn to_raw_value_works() {
    #[derive(Serialize)]
    #[serde(crate = "serde_crate")]
    struct Payload<'a> {
        name: &'a str,
        ok: bool,
        count: u32,
    }

    let payload = Payload {
        name: "node-1",
        ok: true,
        count: 7,
    };

    let raw = to_raw_value(&payload).unwrap();
    assert!(raw.get().contains("\"name\":\"node-1\""));
    assert!(raw.get().contains("\"ok\":true"));
    assert!(raw.get().contains("\"count\":7"));
}

#[test]
fn parse_error_shape_for_selected_cases() {
    let err = from_str::<Value>("[1,]").unwrap_err();
    assert!(err.to_string().contains("expected") || err.to_string().contains("unexpected"));

    let err = from_str::<Value>("\"\\uD83C\\uFFFF\"").unwrap_err();
    assert!(err.to_string().contains("unicode") || err.to_string().contains("invalid"));
}

#[test]
fn preserve_order_parse_works() {
    let input = r#"{"z":0,"a":1,"m":2}"#;

    let value: Value = from_str(input).unwrap();
    let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();

    assert_eq!(keys, vec!["z", "a", "m"]);
}

#[test]
fn preserve_order_map_mutations_work() {
    let mut map = Map::new();
    map.insert("a".into(), 1.into());
    map.insert("b".into(), 2.into());
    map.insert("c".into(), 3.into());

    // Test swap_remove
    let removed = map.swap_remove("a");
    assert_eq!(removed, Some(1.into()));
    let keys: Vec<_> = map.keys().cloned().collect();
    // swap_remove swaps the last element into the removed position
    assert_eq!(keys, vec!["c", "b"]);

    // Test sort_keys
    let mut map2 = Map::new();
    map2.insert("z".into(), 0.into());
    map2.insert("a".into(), 1.into());
    map2.insert("m".into(), 2.into());
    map2.sort_keys();
    let keys: Vec<_> = map2.keys().cloned().collect();
    assert_eq!(keys, vec!["a", "m", "z"]);
}

#[test]
fn serializer_output_works() {
    let cases = [
        r#"null"#,
        r#"true"#,
        r#"123"#,
        r#""hello\nworld""#,
        r#"[1,true,{"a":[null,"x"]}]"#,
        r#"{"z":0,"a":[1,2,3],"nested":{"ok":true,"msg":"hi"}}"#,
    ];

    for input in cases {
        let value: Value = from_str(input).unwrap();
        let output = to_string(&value).unwrap();
        // Re-parse to compare structure
        let reparsed: Value = from_str(&output).unwrap();
        assert_eq!(
            to_string(&value).unwrap(),
            to_string(&reparsed).unwrap(),
            "input={input}"
        );
    }
}

#[test]
fn parse_roundtrip_works() {
    let cases = [
        r#"{"a":1,"b":[true,false,null],"c":{"x":"y"}}"#,
        r#"[{"id":1},{"id":2,"tags":["a","b"]}]"#,
        r#"{"unicode":"\u2603","escaped":"line\nbreak","num":-12.5}"#,
    ];

    for input in cases {
        let value: Value = from_str(input).unwrap();
        let output = to_string(&value).unwrap();
        let reparsed: Value = from_str(&output).unwrap();
        assert_eq!(
            to_string(&value).unwrap(),
            to_string(&reparsed).unwrap(),
            "input={input}"
        );
    }
}
