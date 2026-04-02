// Adapted from serde_json/tests/map.rs
use lifegraph_json::{from_str, Map, Value};

#[test]
fn test_insertion_order() {
    // lifegraph-json preserves insertion order by default
    const EXPECTED: &[&str] = &["b", "a", "c"];

    let v: Value = from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
    let keys: Vec<_> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, EXPECTED);
}

#[test]
fn test_append() {
    // lifegraph-json preserves insertion order
    const EXPECTED: &[&str] = &["b", "a", "c"];

    let mut v: Value = from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
    let val = v.as_object_mut().unwrap();
    let mut m = Map::new();
    m.append(val);
    let keys: Vec<_> = m.keys().collect();

    assert_eq!(keys, EXPECTED);
    assert!(val.is_empty());
}

#[test]
fn test_retain() {
    let mut v: Value = from_str(r#"{"b":null,"a":null,"c":null}"#).unwrap();
    let val = v.as_object_mut().unwrap();
    val.retain(|k, _| k.as_str() != "b");

    let keys: Vec<_> = val.keys().collect();
    assert_eq!(keys, &["a", "c"]);
}
