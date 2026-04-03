mod borrowed_value;
mod error;
mod index;
mod json_macro;
mod map;
mod number;
mod parse;
mod partial_eq;
#[cfg(all(feature = "serde", feature = "raw_value"))]
mod raw;
mod serde_api;
#[cfg(feature = "serde")]
mod serde_deserialize;
#[cfg(feature = "serde")]
mod serde_error;
#[cfg(feature = "serde")]
mod serde_serialize;
mod tape;
mod util;
mod value;

pub use borrowed_value::BorrowedJsonValue;
pub use error::{JsonError, JsonParseError};
pub use index::ValueIndex;
pub use map::Map;
pub use number::JsonNumber;
pub use serde_api::{
    escape_json_string, from_reader, from_slice, from_str, parse_json, parse_json_borrowed,
    parse_json_tape, to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer,
    to_writer_pretty,
};
#[cfg(feature = "serde")]
pub use serde_api::{from_value, to_value};
pub use tape::{
    CompiledObjectSchema, CompiledRowSchema, CompiledTapeKey, CompiledTapeKeys, IndexedTapeObject,
    JsonTape, TapeObjectIndex, TapeToken, TapeTokenKind, TapeValue,
};
pub use value::{JsonValue, Number, Value};

#[cfg(all(feature = "serde", feature = "raw_value"))]
pub use raw::{to_raw_value, RawValue};
#[cfg(feature = "serde")]
pub use serde_deserialize::JsonValueDeserializer;
#[cfg(feature = "serde")]
pub use serde_error::{Category, Error};

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

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
        assert_eq!(
            got,
            vec![Some(TapeTokenKind::String), Some(TapeTokenKind::Bool), None]
        );
    }

    #[test]
    #[cfg(not(feature = "serde"))]
    fn serde_style_convenience_api_works() {
        let value = from_str(r#"{"ok":true,"n":7,"items":[1,2,3],"msg":"hello"}"#).unwrap();
        assert!(value.is_object());
        assert_eq!(value["ok"].as_bool(), Some(true));
        assert_eq!(value["n"].as_i64(), Some(7));
        assert_eq!(value["msg"].as_str(), Some("hello"));
        assert_eq!(value["items"][1].as_u64(), Some(2));
        assert!(value["missing"].is_null());
        assert_eq!(
            to_string(&value).unwrap(),
            r#"{"ok":true,"n":7,"items":[1,2,3],"msg":"hello"}"#
        );
        assert_eq!(
            from_slice(br#"[1,true,"x"]"#).unwrap()[2].as_str(),
            Some("x")
        );
        assert_eq!(
            to_vec(&value).unwrap(),
            value.to_json_string().unwrap().into_bytes()
        );
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_style_convenience_api_works() {
        let value: JsonValue =
            from_str(r#"{"ok":true,"n":7,"items":[1,2,3],"msg":"hello"}"#).unwrap();
        assert!(value.is_object());
        assert_eq!(value["ok"].as_bool(), Some(true));
        assert_eq!(value["n"].as_i64(), Some(7));
        assert_eq!(value["msg"].as_str(), Some("hello"));
        assert_eq!(value["items"][1].as_u64(), Some(2));
        assert!(value["missing"].is_null());
        assert_eq!(
            to_string(&value).unwrap(),
            r#"{"ok":true,"n":7,"items":[1,2,3],"msg":"hello"}"#
        );
        let arr: JsonValue = from_slice(br#"[1,true,"x"]"#).unwrap();
        assert_eq!(arr[2].as_str(), Some("x"));
        assert_eq!(
            to_vec(&value).unwrap(),
            value.to_json_string().unwrap().into_bytes()
        );
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
    #[cfg(not(feature = "serde"))]
    fn from_slice_rejects_invalid_utf8() {
        assert!(matches!(
            from_slice(&[0xff]),
            Err(JsonParseError::InvalidUtf8)
        ));
    }

    #[test]
    #[cfg(not(feature = "serde"))]
    fn pointer_take_and_pretty_helpers_work() {
        let mut value = from_str(r#"{"a":{"b":[10,20,{"~key/":"x"}]}}"#).unwrap();
        assert_eq!(
            value.pointer("/a/b/1").and_then(JsonValue::as_u64),
            Some(20)
        );
        assert_eq!(
            value.pointer("/a/b/2/~0key~1").and_then(JsonValue::as_str),
            Some("x")
        );
        *value.pointer_mut("/a/b/0").unwrap() = JsonValue::from(99u64);
        assert_eq!(
            value.pointer("/a/b/0").and_then(JsonValue::as_u64),
            Some(99)
        );

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
    #[cfg(not(feature = "serde"))]
    fn reader_writer_and_collection_helpers_work() {
        let value = from_reader(std::io::Cursor::new(
            br#"{"a":1,"b":[true,false]}"# as &[u8],
        ))
        .unwrap();
        assert_eq!(value["a"].as_u64(), Some(1));
        assert_eq!(value["b"].len(), 2);
        assert_eq!(
            value["b"].get_index(1).and_then(JsonValue::as_bool),
            Some(false)
        );

        let mut out = Vec::new();
        to_writer(&mut out, &value).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            value.to_json_string().unwrap()
        );

        let object = JsonValue::from_iter([("x", 1u64), ("y", 2u64)]);
        assert_eq!(object["x"].as_u64(), Some(1));
        let array = JsonValue::from_iter([1u64, 2u64, 3u64]);
        assert_eq!(array.get_index(2).and_then(JsonValue::as_u64), Some(3));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn reader_writer_and_collection_helpers_work() {
        let value: JsonValue = from_reader(std::io::Cursor::new(
            br#"{"a":1,"b":[true,false]}"# as &[u8],
        ))
        .unwrap();
        assert_eq!(value["a"].as_u64(), Some(1));
        assert_eq!(value["b"].len(), 2);
        assert_eq!(
            value["b"].get_index(1).and_then(JsonValue::as_bool),
            Some(false)
        );

        let mut out = Vec::new();
        to_writer(&mut out, &value).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            value.to_json_string().unwrap()
        );

        let object = JsonValue::from_iter([("x", 1u64), ("y", 2u64)]);
        assert_eq!(object["x"].as_u64(), Some(1));
        let array = JsonValue::from_iter([1u64, 2u64, 3u64]);
        assert_eq!(array.get_index(2).and_then(JsonValue::as_u64), Some(3));
    }

    #[test]
    fn positive_signed_integer_construction_matches_serde_style() {
        let value = JsonValue::from(64i64);
        assert!(value.is_i64());
        assert!(value.is_u64());
        assert_eq!(value.as_i64(), Some(64));
        assert_eq!(value.as_u64(), Some(64));
        assert_eq!(value.as_number(), Some(&JsonNumber::from(64i64)));
    }

    #[test]
    fn json_number_parity_helpers_work() {
        let n = JsonNumber::from_i128(42).unwrap();
        assert!(n.is_i64() || n.is_u64());
        assert_eq!(n.as_i128(), Some(42));
        assert_eq!(n.as_u128(), Some(42));
        assert_eq!(n.to_string(), "42");

        let big = JsonNumber::from_u128(u128::from(u64::MAX) + 1);
        assert!(big.is_none());
        assert!(JsonNumber::from_f64(f64::NAN).is_none());
    }

    #[test]
    fn json_macro_expr_key_parity_works() {
        let code = 200;
        let features = ["serde", "json"];
        let value = json!({
            "code": code,
            "success": code == 200,
            features[0]: features[1],
        });
        assert_eq!(value["code"], 200);
        assert_eq!(value["success"], true);
        assert_eq!(value["serde"], "json");
    }

    #[test]
    fn primitive_partial_eq_parity_works() {
        let value = json!({"n": 1, "f": 2.5, "b": true, "s": "x"});
        assert_eq!(value["n"], 1);
        assert_eq!(1, value["n"]);
        assert_eq!(value["f"], 2.5);
        assert_eq!(value["b"], true);
        assert_eq!(value["s"], "x");
        assert_eq!(String::from("x"), value["s"]);
    }

    #[test]
    fn signature_and_sort_parity_helpers_work() {
        let mut value = json!({"z": {"b": 2, "a": 1}, "a": [{"d": 4, "c": 3}]});
        assert_eq!(value.as_object().unwrap().len(), 2);
        assert_eq!(value["a"].as_array().unwrap().len(), 1);
        value.sort_all_objects();
        let root_keys = value
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, _)| k.as_str())
            .collect::<Vec<_>>();
        assert_eq!(root_keys, vec!["a", "z"]);
        let nested_keys = value["z"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, _)| k.as_str())
            .collect::<Vec<_>>();
        assert_eq!(nested_keys, vec!["a", "b"]);
    }

    #[test]
    fn generic_get_and_get_mut_index_parity_work() {
        let mut value = json!({"obj": {"x": 1}, "arr": [10, 20, 30]});
        let key = String::from("obj");
        assert_eq!(
            value
                .get("obj")
                .and_then(|v| v.get("x"))
                .and_then(JsonValue::as_u64),
            Some(1)
        );
        assert_eq!(
            value
                .get(&key)
                .and_then(|v| v.get("x"))
                .and_then(JsonValue::as_u64),
            Some(1)
        );
        assert_eq!(
            value
                .get("arr")
                .and_then(|v| v.get(1))
                .and_then(JsonValue::as_u64),
            Some(20)
        );
        *value.get_mut("arr").unwrap().get_mut(2).unwrap() = JsonValue::from(99u64);
        assert_eq!(value["arr"][2].as_u64(), Some(99));
    }

    #[test]
    fn number_and_mut_index_parity_helpers_work() {
        let int = JsonValue::from(7i64);
        assert!(int.is_i64());
        assert!(int.is_u64());
        assert!(!int.is_f64());
        assert_eq!(int.as_number().and_then(JsonNumber::as_i64), Some(7));

        let float = JsonValue::Number(JsonNumber::from_f64(2.5).unwrap());
        assert!(float.is_f64());
        assert_eq!(float.as_f64(), Some(2.5));
        assert_eq!(JsonValue::Null.as_null(), Some(()));

        let mut value = JsonValue::Null;
        value["a"]["b"]["c"] = JsonValue::from(true);
        assert_eq!(
            value.pointer("/a/b/c").and_then(JsonValue::as_bool),
            Some(true)
        );

        value["arr"] = json!([1, 2, 3]);
        value["arr"][1] = JsonValue::from(9u64);
        assert_eq!(value.pointer("/arr/1").and_then(JsonValue::as_u64), Some(9));
    }

    #[test]
    fn map_get_insert_remove_and_contains_key_work() {
        let mut map = Map::new();
        assert_eq!(map.insert("x".to_owned(), JsonValue::from(1u64)), None);
        assert!(map.contains_key("x"));
        assert_eq!(map.get("x").and_then(JsonValue::as_u64), Some(1));
        *map.get_mut("x").unwrap() = JsonValue::from(2u64);
        assert_eq!(map.get("x").and_then(JsonValue::as_u64), Some(2));
        assert_eq!(
            map.insert("x".to_owned(), JsonValue::from(3u64))
                .and_then(|v| v.as_u64()),
            Some(2)
        );
        assert_eq!(map.remove("x").and_then(|v| v.as_u64()), Some(3));
        assert!(!map.contains_key("x"));
    }

    #[test]
    fn serde_map_style_tests_work() {
        let v: Value = from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, vec!["b", "a", "c"]);

        let mut v: Value = from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
        let val = v.as_object_mut().unwrap();
        let mut m = Map::new();
        m.append(val);
        let keys: Vec<_> = m.keys().cloned().collect();
        assert_eq!(keys, vec!["b", "a", "c"]);
        assert!(val.is_empty());

        let mut v: Value = from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
        let val = v.as_object_mut().unwrap();
        val.retain(|k, _| k.as_str() != "b");
        let keys: Vec<_> = val.keys().cloned().collect();
        assert_eq!(keys, vec!["a", "c"]);
    }

    #[test]
    fn serde_value_doc_examples_get_and_index_work() {
        let object = json!({"A": 65, "B": 66, "C": 67});
        assert_eq!(*object.get("A").unwrap(), json!(65));

        let array = json!(["A", "B", "C"]);
        assert_eq!(*array.get(2).unwrap(), json!("C"));
        assert_eq!(array.get("A"), None);

        let object = json!({"A": ["a", "á", "à"], "B": ["b", "b́"], "C": ["c", "ć", "ć̣", "ḉ"]});
        assert_eq!(object["B"][0], json!("b"));
        assert_eq!(object["D"], json!(null));
        assert_eq!(object[0]["x"]["y"]["z"], json!(null));
    }

    #[test]
    fn serde_value_doc_examples_type_queries_work() {
        let obj = json!({ "a": { "nested": true }, "b": ["an", "array"] });
        assert!(obj.is_object());
        assert!(obj["a"].is_object());
        assert!(!obj["b"].is_object());
        assert!(obj["b"].is_array());
        assert!(!obj["a"].is_array());

        let v = json!({ "a": "some string", "b": false });
        assert!(v["a"].is_string());
        assert!(!v["b"].is_string());

        let v = json!({ "a": 1, "b": "2" });
        assert!(v["a"].is_number());
        assert!(!v["b"].is_number());

        let v = json!({ "a": false, "b": "false" });
        assert!(v["a"].is_boolean());
        assert!(!v["b"].is_boolean());

        let v = json!({ "a": null, "b": false });
        assert!(v["a"].is_null());
        assert!(!v["b"].is_null());
    }

    #[test]
    fn serde_value_doc_examples_accessors_work() {
        let v = json!({ "a": { "nested": true }, "b": ["an", "array"] });
        assert_eq!(v["a"].as_object().unwrap().len(), 1);
        assert_eq!(v["b"].as_array().unwrap().len(), 2);
        assert_eq!(v["b"].as_object(), None);

        let v = json!({ "a": "some string", "b": false });
        assert_eq!(v["a"].as_str(), Some("some string"));
        assert_eq!(v["b"].as_str(), None);

        let v = json!({ "a": 1, "b": 2.2, "c": -3, "d": "4" });
        assert_eq!(v["a"].as_number(), Some(&JsonNumber::from(1u64)));
        assert_eq!(
            v["b"].as_number(),
            Some(&JsonNumber::from_f64(2.2).unwrap())
        );
        assert_eq!(v["c"].as_number(), Some(&JsonNumber::from(-3i64)));
        assert_eq!(v["d"].as_number(), None);
    }

    #[test]
    fn serde_value_doc_examples_numeric_queries_work() {
        let big = i64::MAX as u64 + 10;
        let v = json!({ "a": 64, "b": big, "c": 256.0 });
        assert!(v["a"].is_i64());
        assert!(!v["b"].is_i64());
        assert!(!v["c"].is_i64());
        assert_eq!(v["a"].as_i64(), Some(64));
        assert_eq!(v["b"].as_i64(), None);
        assert_eq!(v["c"].as_i64(), None);

        let v = json!({ "a": 64, "b": -64, "c": 256.0 });
        assert!(v["a"].is_u64());
        assert!(!v["b"].is_u64());
        assert!(!v["c"].is_u64());
        assert_eq!(v["a"].as_u64(), Some(64));
        assert_eq!(v["b"].as_u64(), None);
        assert_eq!(v["c"].as_u64(), None);

        let v = json!({ "a": 256.0, "b": 64, "c": -64 });
        assert!(v["a"].is_f64());
        assert!(!v["b"].is_f64());
        assert!(!v["c"].is_f64());
        assert_eq!(v["a"].as_f64(), Some(256.0));
        assert_eq!(v["b"].as_f64(), Some(64.0));
        assert_eq!(v["c"].as_f64(), Some(-64.0));
    }

    #[test]
    fn serde_value_doc_examples_pointer_and_take_work() {
        let data = json!({
            "x": {
                "y": ["z", "zz"]
            }
        });
        assert_eq!(data.pointer("/x/y/1").unwrap(), &json!("zz"));
        assert_eq!(data.pointer("/a/b/c"), None);

        let mut value = json!({"x": 1.0, "y": 2.0});
        assert_eq!(value.pointer("/x"), Some(&JsonValue::from(1.0)));
        if let Some(v) = value.pointer_mut("/x") {
            *v = 1.5.into();
        }
        assert_eq!(value.pointer("/x"), Some(&JsonValue::from(1.5)));
        let old_x = value.pointer_mut("/x").map(JsonValue::take).unwrap();
        assert_eq!(old_x, JsonValue::from(1.5));
        assert_eq!(value.pointer("/x").unwrap(), &JsonValue::Null);

        let mut v = json!({"x": "y"});
        assert_eq!(v["x"].take(), json!("y"));
        assert_eq!(v, json!({"x": null}));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_pure_json_parse_examples_work() {
        assert_eq!(from_str::<JsonValue>("null").unwrap(), json!(null));
        assert_eq!(from_str::<JsonValue>(" true ").unwrap(), json!(true));
        assert_eq!(from_str::<JsonValue>(" false ").unwrap(), json!(false));
        assert_eq!(from_str::<JsonValue>(r#""foo""#).unwrap(), json!("foo"));
        assert_eq!(
            from_str::<JsonValue>(r#""\uD83C\uDF95""#).unwrap(),
            json!("🎕")
        );
        assert_eq!(from_str::<JsonValue>("[]").unwrap(), json!([]));
        assert_eq!(
            from_str::<JsonValue>("[1, [2, 3]]").unwrap(),
            json!([1, [2, 3]])
        );
        assert_eq!(from_str::<JsonValue>("{}").unwrap(), json!({}));
        assert_eq!(
            from_str::<JsonValue>(r#"{"a": {"b": 3, "c": 4}}"#).unwrap(),
            json!({"a": {"b": 3, "c": 4}})
        );

        let neg_zero: JsonValue = from_str("-0.0").unwrap();
        let parsed = neg_zero.as_f64().unwrap();
        assert!(parsed.is_sign_negative());
    }

    #[test]
    #[cfg(not(feature = "serde"))]
    fn parser_regression_cases_work() {
        assert!(matches!(
            from_str("+"),
            Err(JsonParseError::UnexpectedCharacter { .. })
        ));
        assert!(matches!(
            from_str("."),
            Err(JsonParseError::UnexpectedCharacter { .. })
        ));
        assert!(matches!(
            from_str("-"),
            Err(JsonParseError::UnexpectedEnd)
                | Err(JsonParseError::InvalidNumber { .. })
                | Err(JsonParseError::UnexpectedCharacter { .. })
        ));
        assert!(matches!(
            from_str("00"),
            Err(JsonParseError::InvalidNumber { .. })
        ));
        assert!(matches!(
            from_str("0."),
            Err(JsonParseError::UnexpectedEnd) | Err(JsonParseError::InvalidNumber { .. })
        ));
        assert!(matches!(
            from_str("1e"),
            Err(JsonParseError::UnexpectedEnd) | Err(JsonParseError::InvalidNumber { .. })
        ));
        assert!(matches!(
            from_str("1e+"),
            Err(JsonParseError::UnexpectedEnd) | Err(JsonParseError::InvalidNumber { .. })
        ));
        assert!(matches!(
            from_str("1a"),
            Err(JsonParseError::UnexpectedTrailingCharacters(_))
        ));
        assert!(matches!(
            from_str("[1,]"),
            Err(JsonParseError::UnexpectedCharacter { .. })
                | Err(JsonParseError::UnexpectedEnd)
                | Err(JsonParseError::ExpectedCommaOrEnd { .. })
        ));
        assert!(matches!(
            from_str("[1 2]"),
            Err(JsonParseError::ExpectedCommaOrEnd { .. })
        ));
        assert!(matches!(
            from_str(r#"{"a":1 1"#),
            Err(JsonParseError::ExpectedCommaOrEnd { .. })
        ));
        assert!(matches!(
            from_str(r#"{"a":1,"#),
            Err(JsonParseError::UnexpectedEnd) | Err(JsonParseError::UnexpectedCharacter { .. })
        ));
        assert!(matches!(
            from_str("{1"),
            Err(JsonParseError::UnexpectedCharacter { .. })
        ));
        assert!(matches!(
            from_str(r#""\uD83C\uFFFF""#),
            Err(JsonParseError::InvalidUnicodeScalar { .. })
        ));
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
                JsonValue::Object(Map::from(vec![("x".into(), 1u64.into())])),
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
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
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
            0..=3 => random_leaf(rng),
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
                JsonValue::Object(Map::from(entries))
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
