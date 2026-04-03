#![feature(test)]
extern crate test;

#[cfg(feature = "serde")]
use lifegraph_json::{from_str, to_value};
use lifegraph_json::{
    parse_json, parse_json_borrowed, parse_json_tape, to_string, to_vec, CompiledTapeKeys,
    JsonValue, Map,
};
use test::{black_box, Bencher};

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
    JsonValue::Array((0..100).map(JsonValue::from).collect())
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

#[bench]
fn bench_serialize_small_object(b: &mut Bencher) {
    let obj = small_object();
    b.iter(|| black_box(to_string(&obj)).unwrap());
}

#[bench]
fn bench_serialize_medium_object(b: &mut Bencher) {
    let obj = medium_object();
    b.iter(|| black_box(to_string(&obj)).unwrap());
}

#[bench]
fn bench_serialize_array_of_ints(b: &mut Bencher) {
    let obj = array_of_ints();
    b.iter(|| black_box(to_string(&obj)).unwrap());
}

#[bench]
fn bench_serialize_nested_arrays(b: &mut Bencher) {
    let obj = nested_arrays();
    b.iter(|| black_box(to_string(&obj)).unwrap());
}

#[bench]
fn bench_deserialize_small_object(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let _: JsonValue = black_box(parse_json(json)).unwrap();
    });
}

#[bench]
fn bench_deserialize_medium_object(b: &mut Bencher) {
    let json = r#"{
        "user": {
            "id": 12345,
            "name": "John Doe",
            "email": "john@example.com",
            "roles": ["admin", "user", "editor"],
            "metadata": {
                "created": 1234567890,
                "updated": 9876543210,
                "tags": ["a", "b", "c", "d", "e"]
            }
        },
        "status": "active",
        "count": 100
    }"#;
    b.iter(|| {
        let _: JsonValue = black_box(parse_json(json)).unwrap();
    });
}

#[bench]
fn bench_deserialize_array_of_ints(b: &mut Bencher) {
    let json: String = format!(
        "[{}]",
        (0..100)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    b.iter(|| {
        let _: JsonValue = black_box(parse_json(&json)).unwrap();
    });
}

#[bench]
fn bench_serialize_string_with_escapes(b: &mut Bencher) {
    let obj = string_with_escapes();
    b.iter(|| black_box(to_string(&obj)).unwrap());
}

#[bench]
fn bench_to_vec_small_object(b: &mut Bencher) {
    let obj = small_object();
    b.iter(|| black_box(to_vec(&obj)).unwrap());
}

#[bench]
fn bench_roundtrip_small(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let parsed: JsonValue = black_box(parse_json(json)).unwrap();
        black_box(to_string(&parsed)).unwrap();
    });
}

#[bench]
fn bench_roundtrip_medium(b: &mut Bencher) {
    let json = r#"{
        "user": {
            "id": 12345,
            "name": "John Doe",
            "email": "john@example.com",
            "roles": ["admin", "user", "editor"],
            "metadata": {
                "created": 1234567890,
                "updated": 9876543210,
                "tags": ["a", "b", "c", "d", "e"]
            }
        },
        "status": "active",
        "count": 100
    }"#;
    b.iter(|| {
        let parsed: JsonValue = black_box(parse_json(json)).unwrap();
        black_box(to_string(&parsed)).unwrap();
    });
}

// ============= TAPE AND BORROWED PATH BENCHMARKS =============

#[bench]
fn bench_tape_parse_small(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let tape = black_box(parse_json_tape(json)).unwrap();
        black_box(tape.tokens.len());
    });
}

#[bench]
fn bench_tape_parse_medium(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#;
    b.iter(|| {
        let tape = black_box(parse_json_tape(json)).unwrap();
        black_box(tape.tokens.len());
    });
}

#[bench]
fn bench_borrowed_parse_small(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let _val = black_box(parse_json_borrowed(json)).unwrap();
    });
}

#[bench]
fn bench_borrowed_parse_medium(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#;
    b.iter(|| {
        let _val = black_box(parse_json_borrowed(json)).unwrap();
    });
}

#[bench]
fn bench_tape_indexed_lookup_small(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    let tape = parse_json_tape(json).unwrap();
    let root = tape.root(json).unwrap();
    let index = root.build_object_index().unwrap();
    let indexed = root.with_index(&index);
    let keys = CompiledTapeKeys::new(&["id", "name", "active"]);
    b.iter(|| {
        let vals: Vec<_> = indexed.get_compiled_many(&keys).collect();
        black_box(vals.len());
    });
}

#[bench]
fn bench_tape_indexed_lookup_medium(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#;
    let tape = parse_json_tape(json).unwrap();
    let root = tape.root(json).unwrap();
    let index = root.build_object_index().unwrap();
    let indexed = root.with_index(&index);
    let keys = CompiledTapeKeys::new(&["user", "status", "count"]);
    b.iter(|| {
        let vals: Vec<_> = indexed.get_compiled_many(&keys).collect();
        black_box(vals.len());
    });
}

#[bench]
fn bench_repeated_tape_parse_plus_lookup(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com"},"status":"active","count":100}"#;
    let keys = CompiledTapeKeys::new(&["status", "count"]);
    b.iter(|| {
        let tape = parse_json_tape(json).unwrap();
        let root = tape.root(json).unwrap();
        let index = root.build_object_index().unwrap();
        let indexed = root.with_index(&index);
        let vals: Vec<_> = indexed.get_compiled_many(&keys).collect();
        black_box(vals.len());
    });
}

// ---------------------------------------------------------------------------
// Serde integration benchmarks (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[bench]
fn bench_serde_from_str_small_object(b: &mut Bencher) {
    let json = r#"{"id":42,"name":"test","active":true}"#;
    b.iter(|| {
        let _: JsonValue = black_box(from_str(json)).unwrap();
    });
}

#[cfg(feature = "serde")]
#[bench]
fn bench_serde_from_str_medium_object(b: &mut Bencher) {
    let json = r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#;
    b.iter(|| {
        let _: JsonValue = black_box(from_str(json)).unwrap();
    });
}

#[cfg(feature = "serde")]
#[bench]
fn bench_serde_to_value_small(b: &mut Bencher) {
    let obj = small_object();
    b.iter(|| {
        let _: JsonValue = black_box(to_value(&obj)).unwrap();
    });
}
