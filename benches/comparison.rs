#![cfg_attr(feature = "serde_json_bench", feature(test))]
#![cfg(feature = "serde_json_bench")]
extern crate test;

use serde_json::{from_str, to_string, to_vec, JsonValue, Map};
#[cfg(feature = "serde_json_bench")]
use serde_json_upstream;
use test::{black_box, Bencher};

// ============= SERIALIZATION BENCHMARKS =============

#[bench]
fn serialize_small_object_lifegraph(b: &mut Bencher) {
    let obj = small_object();
    b.iter(|| black_box(to_string(&obj)).unwrap())
}

#[bench]
fn serialize_small_object_upstream(b: &mut Bencher) {
    let obj = small_object_upstream();
    b.iter(|| black_box(serde_json_upstream::to_string(&obj)).unwrap())
}

#[bench]
fn serialize_medium_object_lifegraph(b: &mut Bencher) {
    let obj = medium_object();
    b.iter(|| black_box(to_string(&obj)).unwrap())
}

#[bench]
fn serialize_medium_object_upstream(b: &mut Bencher) {
    let obj = medium_object_upstream();
    b.iter(|| black_box(serde_json_upstream::to_string(&obj)).unwrap())
}

#[bench]
fn serialize_array_of_ints_lifegraph(b: &mut Bencher) {
    let obj = array_of_ints();
    b.iter(|| black_box(to_string(&obj)).unwrap())
}

#[bench]
fn serialize_array_of_ints_upstream(b: &mut Bencher) {
    let obj = array_of_ints_upstream();
    b.iter(|| black_box(serde_json_upstream::to_string(&obj)).unwrap())
}

#[bench]
fn serialize_nested_arrays_lifegraph(b: &mut Bencher) {
    let obj = nested_arrays();
    b.iter(|| black_box(to_string(&obj)).unwrap())
}

#[bench]
fn serialize_nested_arrays_upstream(b: &mut Bencher) {
    let obj = nested_arrays_upstream();
    b.iter(|| black_box(serde_json_upstream::to_string(&obj)).unwrap())
}

#[bench]
fn serialize_string_with_escapes_lifegraph(b: &mut Bencher) {
    let obj = string_with_escapes();
    b.iter(|| black_box(to_string(&obj)).unwrap())
}

#[bench]
fn serialize_string_with_escapes_upstream(b: &mut Bencher) {
    let obj = string_with_escapes_upstream();
    b.iter(|| black_box(serde_json_upstream::to_string(&obj)).unwrap())
}

// ============= DESERIALIZATION BENCHMARKS =============

#[bench]
fn deserialize_small_object_lifegraph(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let _: JsonValue = black_box(from_str(json)).unwrap();
    })
}

#[bench]
fn deserialize_small_object_upstream(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let _: serde_json_upstream::Value = black_box(serde_json_upstream::from_str(json)).unwrap();
    })
}

#[bench]
fn deserialize_medium_object_lifegraph(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#;
    b.iter(|| {
        let _: JsonValue = black_box(from_str(json)).unwrap();
    })
}

#[bench]
fn deserialize_medium_object_upstream(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#;
    b.iter(|| {
        let _: serde_json_upstream::Value = black_box(serde_json_upstream::from_str(json)).unwrap();
    })
}

#[bench]
fn deserialize_array_of_ints_lifegraph(b: &mut Bencher) {
    let json: String = format!(
        "[{}]",
        (0..100)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    b.iter(|| {
        let _: JsonValue = black_box(from_str(&json)).unwrap();
    })
}

#[bench]
fn deserialize_array_of_ints_upstream(b: &mut Bencher) {
    let json: String = format!(
        "[{}]",
        (0..100)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    b.iter(|| {
        let _: serde_json_upstream::Value =
            black_box(serde_json_upstream::from_str(&json)).unwrap();
    })
}

// ============= ROUNDTRIP BENCHMARKS =============

#[bench]
fn roundtrip_small_lifegraph(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let parsed: JsonValue = black_box(from_str(json)).unwrap();
        black_box(to_string(&parsed)).unwrap();
    })
}

#[bench]
fn roundtrip_small_upstream(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let parsed: serde_json_upstream::Value =
            black_box(serde_json_upstream::from_str(json)).unwrap();
        black_box(serde_json_upstream::to_string(&parsed)).unwrap();
    })
}

#[bench]
fn roundtrip_medium_lifegraph(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#;
    b.iter(|| {
        let parsed: JsonValue = black_box(from_str(json)).unwrap();
        black_box(to_string(&parsed)).unwrap();
    })
}

#[bench]
fn roundtrip_medium_upstream(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#;
    b.iter(|| {
        let parsed: serde_json_upstream::Value =
            black_box(serde_json_upstream::from_str(json)).unwrap();
        black_box(serde_json_upstream::to_string(&parsed)).unwrap();
    })
}

// ============= HELPERS (LIFEGRAF) =============

fn small_object() -> JsonValue {
    let mut obj = JsonValue::Object(Map::new());
    obj.push_field("id", 42);
    obj.push_field("name", "test");
    obj.push_field("active", true);
    obj
}

fn medium_object() -> JsonValue {
    let mut metadata = JsonValue::Object(Map::new());
    metadata.push_field("created", 1234567890i64);
    metadata.push_field("updated", 9876543210i64);
    metadata.push_field(
        "tags",
        JsonValue::Array(
            vec!["a", "b", "c", "d", "e"]
                .into_iter()
                .map(|s| JsonValue::String(s.to_string()))
                .collect(),
        ),
    );

    let mut user = JsonValue::Object(Map::new());
    user.push_field("id", 12345i64);
    user.push_field("name", "John Doe");
    user.push_field("email", "john@example.com");
    user.push_field(
        "roles",
        JsonValue::Array(
            vec!["admin", "user", "editor"]
                .into_iter()
                .map(|s| JsonValue::String(s.to_string()))
                .collect(),
        ),
    );
    user.push_field("metadata", metadata);

    let mut obj = JsonValue::Object(Map::new());
    obj.push_field("user", user);
    obj.push_field("status", "active");
    obj.push_field("count", 100i64);
    obj
}

fn array_of_ints() -> JsonValue {
    JsonValue::Array((0..100).map(|i| JsonValue::from(i)).collect())
}

fn nested_arrays() -> JsonValue {
    JsonValue::Array(
        (0..50)
            .map(|i| JsonValue::Array((0..10).map(|j| JsonValue::from(i * 10 + j)).collect()))
            .collect(),
    )
}

fn string_with_escapes() -> JsonValue {
    let mut obj = JsonValue::Object(Map::new());
    obj.push_field(
        "message",
        "Hello \"World\"!\nLine\tbreak\rwith\tspecial\u{0001}chars",
    );
    obj
}

// ============= HELPERS (UPSTREAM) =============

fn small_object_upstream() -> serde_json_upstream::Value {
    serde_json_upstream::json!({
        "id": 42,
        "name": "test",
        "active": true
    })
}

fn medium_object_upstream() -> serde_json_upstream::Value {
    serde_json_upstream::json!({
        "user": {
            "id": 12345i64,
            "name": "John Doe",
            "email": "john@example.com",
            "roles": ["admin", "user", "editor"],
            "metadata": {
                "created": 1234567890i64,
                "updated": 9876543210i64,
                "tags": ["a", "b", "c", "d", "e"]
            }
        },
        "status": "active",
        "count": 100i64
    })
}

fn array_of_ints_upstream() -> serde_json_upstream::Value {
    serde_json_upstream::Value::Array((0..100).map(|i| serde_json_upstream::json!(i)).collect())
}

fn nested_arrays_upstream() -> serde_json_upstream::Value {
    serde_json_upstream::Value::Array(
        (0..50)
            .map(|i| {
                serde_json_upstream::Value::Array(
                    (0..10)
                        .map(|j| serde_json_upstream::json!(i * 10 + j))
                        .collect(),
                )
            })
            .collect(),
    )
}

fn string_with_escapes_upstream() -> serde_json_upstream::Value {
    serde_json_upstream::json!({
        "message": "Hello \"World\"!\nLine\tbreak\rwith\tspecial\u{0001}chars"
    })
}
