//! Unified benchmark suite comparing lifegraph-json against serde_json.
//!
//! Requires the `serde_json_bench` feature flag to enable.
//!
//! Real-world usage patterns covered:
//!   - Owned parsing (`parse_json` vs serde_json `from_str::<Value>`)
//!   - Borrowed parsing (`parse_json_borrowed` — lifegraph unique)
//!   - Tape parsing (`parse_json_tape` — lifegraph unique)
//!   - Typed deserialization (`from_str::<T>` vs serde_json `from_str::<T>`)
//!   - Serialization (`to_string` vs serde_json `to_string`)
//!   - Byte paths (`to_vec` / `from_slice` vs serde_json equivalents)
//!   - Deep nesting (recursion depth stress)
//!   - Wide objects (many keys)
//!   - Array-of-objects (tabular data, common in APIs/DBs)
//!   - Roundtrip (parse → serialize)
//!   - Value field access patterns

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use lifegraph_json::{
    from_slice as lg_from_slice, from_str as lg_from_str, parse_json, parse_json_borrowed,
    parse_json_tape, to_string as lg_to_string, to_vec as lg_to_vec, CompiledTapeKeys, JsonValue,
    Map,
};
use serde_crate::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Fixture data constructors
// ---------------------------------------------------------------------------

fn small_json() -> &'static str {
    r#"{"id":42,"name":"test","active":true}"#
}

fn medium_json() -> &'static str {
    r#"{"user":{"id":12345,"name":"John Doe","email":"john@example.com","roles":["admin","user","editor"],"metadata":{"created":1234567890,"updated":9876543210,"tags":["a","b","c","d","e"]}},"status":"active","count":100}"#
}

fn wide_json() -> String {
    let mut pairs: Vec<String> = Vec::with_capacity(100);
    for i in 0..100 {
        pairs.push(format!("\"key{}\":{}", i, i * 7));
    }
    format!("{{{}}}", pairs.join(","))
}

fn deep_json() -> String {
    // Nesting depth of 100 — stresses recursion without hitting limits (max 128)
    let depth = 100;
    let mut s = String::with_capacity(depth * 10);
    for _ in 0..depth {
        s.push_str("{\"a\":");
    }
    s.push_str("1");
    for _ in 0..depth {
        s.push('}');
    }
    s
}

fn large_array_json() -> String {
    // Array of 1000 small objects — simulates API result pages
    let obj = r#"{"id":1,"name":"item","v":3.14}"#;
    let mut s = String::with_capacity(obj.len() * 1000 + 2);
    s.push('[');
    for i in 0..1000 {
        if i > 0 {
            s.push(',');
        }
        s.push_str(
            obj.replace("1", &i.to_string())
                .replace("3.14", &format!("{:.2}", i as f64 * 0.01))
                .as_str(),
        );
    }
    s.push(']');
    s
}

fn tabular_json() -> String {
    // Array of objects with uniform keys — common in database/CSV conversion
    let mut s = String::from('[');
    for i in 0..500 {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            r#"{{"id":{},"name":"user{}","email":"user{}@example.com","score":{}}}"#,
            i,
            i,
            i,
            (i * 17 % 100)
        ));
    }
    s.push(']');
    s
}

fn small_lg_value() -> JsonValue {
    let mut obj = JsonValue::Object(Map::new());
    obj.push_field("id", 42);
    obj.push_field("name", "test");
    obj.push_field("active", true);
    obj
}

fn medium_lg_value() -> JsonValue {
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

// ---------------------------------------------------------------------------
// Typed serde models (real-world pattern: deserialize into structs, not Value)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize)]
#[serde(crate = "serde_crate")]
struct SmallPayload {
    id: u64,
    name: String,
    active: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "serde_crate")]
struct UserMeta {
    created: i64,
    updated: i64,
    tags: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "serde_crate")]
struct User {
    id: i64,
    name: String,
    email: String,
    roles: Vec<String>,
    metadata: UserMeta,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "serde_crate")]
struct MediumPayload {
    user: User,
    status: String,
    count: i64,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "serde_crate")]
struct TabularRow {
    id: u64,
    name: String,
    email: String,
    score: u64,
}

// ---------------------------------------------------------------------------
// 1. OWNED VALUE PARSING (JsonValue vs serde_json::Value)
// ---------------------------------------------------------------------------

fn owned_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("owned_parse");

    group.bench_function(BenchmarkId::new("small", "lifegraph"), |b| {
        let json = small_json();
        b.iter(|| parse_json(json));
    });
    group.bench_function(BenchmarkId::new("small", "serde_json"), |b| {
        let json = small_json();
        b.iter(|| serde_json_upstream::from_str::<serde_json_upstream::Value>(json));
    });

    group.bench_function(BenchmarkId::new("medium", "lifegraph"), |b| {
        let json = medium_json();
        b.iter(|| parse_json(json));
    });
    group.bench_function(BenchmarkId::new("medium", "serde_json"), |b| {
        let json = medium_json();
        b.iter(|| serde_json_upstream::from_str::<serde_json_upstream::Value>(json));
    });

    group.bench_function(BenchmarkId::new("wide", "lifegraph"), |b| {
        let json = wide_json();
        b.iter(|| parse_json(&json));
    });
    group.bench_function(BenchmarkId::new("wide", "serde_json"), |b| {
        let json = wide_json();
        b.iter(|| serde_json_upstream::from_str::<serde_json_upstream::Value>(&json));
    });

    group.bench_function(BenchmarkId::new("deep", "lifegraph"), |b| {
        let json = deep_json();
        b.iter(|| parse_json(&json));
    });
    group.bench_function(BenchmarkId::new("deep", "serde_json"), |b| {
        let json = deep_json();
        b.iter(|| serde_json_upstream::from_str::<serde_json_upstream::Value>(&json));
    });

    group.bench_function(BenchmarkId::new("large_array", "lifegraph"), |b| {
        let json = large_array_json();
        b.iter(|| parse_json(&json));
    });
    group.bench_function(BenchmarkId::new("large_array", "serde_json"), |b| {
        let json = large_array_json();
        b.iter(|| serde_json_upstream::from_str::<serde_json_upstream::Value>(&json));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 2. BORROWED VALUE PARSING (lifegraph unique — zero-copy for strings/keys)
// ---------------------------------------------------------------------------

fn borrowed_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("borrowed_parse");

    group.bench_function("small", |b| {
        let json = small_json();
        b.iter(|| parse_json_borrowed(json));
    });

    group.bench_function("medium", |b| {
        let json = medium_json();
        b.iter(|| parse_json_borrowed(json));
    });

    group.bench_function("large_array", |b| {
        let json = large_array_json();
        b.iter(|| parse_json_borrowed(&json));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 3. TAPE PARSING (lifegraph unique — token stream with indexed lookups)
// ---------------------------------------------------------------------------

fn tape_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("tape_parse");

    group.bench_function(BenchmarkId::new("small", "parse"), |b| {
        let json = small_json();
        b.iter(|| parse_json_tape(json));
    });

    group.bench_function(BenchmarkId::new("medium", "parse"), |b| {
        let json = medium_json();
        b.iter(|| parse_json_tape(json));
    });

    group.bench_function(BenchmarkId::new("large_array", "parse"), |b| {
        let json = large_array_json();
        b.iter(|| parse_json_tape(&json));
    });

    group.bench_function("indexed_lookup", |b| {
        let json = medium_json();
        let tape = parse_json_tape(json).unwrap();
        let root = tape.root(json).unwrap();
        let index = root.build_object_index().unwrap();
        let indexed = root.with_index(&index);
        let keys = CompiledTapeKeys::new(&["user", "status", "count"]);
        b.iter(|| indexed.get_compiled_many(&keys).collect::<Vec<_>>());
    });

    group.bench_function(BenchmarkId::new("field_access", "lifegraph_tape"), |b| {
        let json = medium_json();
        let tape = parse_json_tape(json).unwrap();
        let root = tape.root(json).unwrap();
        let index = root.build_object_index().unwrap();
        let indexed = root.with_index(&index);
        let keys = CompiledTapeKeys::new(&["user", "status", "count"]);
        b.iter(|| indexed.get_compiled_many(&keys).collect::<Vec<_>>());
    });

    group.bench_function(BenchmarkId::new("field_access", "serde_json_value"), |b| {
        let json = medium_json();
        let val: serde_json_upstream::Value = serde_json_upstream::from_str(json).unwrap();
        b.iter(|| {
            let user = val.get("user");
            let status = val.get("status");
            let count = val.get("count");
            (user, status, count)
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 4. TYPED DESERIALIZATION (from_str::<T> — the most common real-world pattern)
// ---------------------------------------------------------------------------

fn typed_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("typed_deserialize");

    group.bench_function(BenchmarkId::new("small", "lifegraph"), |b| {
        let json = small_json();
        b.iter(|| lg_from_str::<SmallPayload>(json));
    });
    group.bench_function(BenchmarkId::new("small", "serde_json"), |b| {
        let json = small_json();
        b.iter(|| serde_json_upstream::from_str::<SmallPayload>(json));
    });

    group.bench_function(BenchmarkId::new("medium", "lifegraph"), |b| {
        let json = medium_json();
        b.iter(|| lg_from_str::<MediumPayload>(json));
    });
    group.bench_function(BenchmarkId::new("medium", "serde_json"), |b| {
        let json = medium_json();
        b.iter(|| serde_json_upstream::from_str::<MediumPayload>(json));
    });

    group.bench_function(BenchmarkId::new("tabular", "lifegraph"), |b| {
        let json = tabular_json();
        b.iter(|| lg_from_str::<Vec<TabularRow>>(&json));
    });
    group.bench_function(BenchmarkId::new("tabular", "serde_json"), |b| {
        let json = tabular_json();
        b.iter(|| serde_json_upstream::from_str::<Vec<TabularRow>>(&json));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 5. SERIALIZATION (to_string — owned JsonValue / serde_json::Value)
// ---------------------------------------------------------------------------

fn serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");

    group.bench_function(BenchmarkId::new("small", "lifegraph"), |b| {
        let obj = small_lg_value();
        b.iter(|| lg_to_string(&obj));
    });
    group.bench_function(BenchmarkId::new("small", "serde_json"), |b| {
        let mut obj = serde_json_upstream::Map::new();
        obj.insert("id".into(), serde_json_upstream::Value::Number(42.into()));
        obj.insert(
            "name".into(),
            serde_json_upstream::Value::String("test".into()),
        );
        obj.insert("active".into(), serde_json_upstream::Value::Bool(true));
        let obj = serde_json_upstream::Value::Object(obj);
        b.iter(|| serde_json_upstream::to_string(&obj));
    });

    group.bench_function(BenchmarkId::new("medium", "lifegraph"), |b| {
        let obj = medium_lg_value();
        b.iter(|| lg_to_string(&obj));
    });
    group.bench_function(BenchmarkId::new("medium", "serde_json"), |b| {
        let mut tags_map = serde_json_upstream::Map::new();
        tags_map.insert(
            "created".into(),
            serde_json_upstream::Value::Number(1234567890i64.into()),
        );
        tags_map.insert(
            "updated".into(),
            serde_json_upstream::Value::Number(9876543210i64.into()),
        );
        tags_map.insert(
            "tags".into(),
            serde_json_upstream::Value::Array(
                vec!["a", "b", "c", "d", "e"]
                    .into_iter()
                    .map(|s| serde_json_upstream::Value::String(s.into()))
                    .collect(),
            ),
        );

        let mut user_map = serde_json_upstream::Map::new();
        user_map.insert(
            "id".into(),
            serde_json_upstream::Value::Number(12345i64.into()),
        );
        user_map.insert(
            "name".into(),
            serde_json_upstream::Value::String("John Doe".into()),
        );
        user_map.insert(
            "email".into(),
            serde_json_upstream::Value::String("john@example.com".into()),
        );
        user_map.insert(
            "roles".into(),
            serde_json_upstream::Value::Array(
                vec!["admin", "user", "editor"]
                    .into_iter()
                    .map(|s| serde_json_upstream::Value::String(s.into()))
                    .collect(),
            ),
        );
        user_map.insert(
            "metadata".into(),
            serde_json_upstream::Value::Object(tags_map),
        );

        let mut outer_map = serde_json_upstream::Map::new();
        outer_map.insert("user".into(), serde_json_upstream::Value::Object(user_map));
        outer_map.insert(
            "status".into(),
            serde_json_upstream::Value::String("active".into()),
        );
        outer_map.insert(
            "count".into(),
            serde_json_upstream::Value::Number(100i64.into()),
        );

        let obj = serde_json_upstream::Value::Object(outer_map);
        b.iter(|| serde_json_upstream::to_string(&obj));
    });

    group.bench_function(BenchmarkId::new("array_of_ints", "lifegraph"), |b| {
        let obj = JsonValue::Array((0..1000).map(JsonValue::from).collect());
        b.iter(|| lg_to_string(&obj));
    });
    group.bench_function(BenchmarkId::new("array_of_ints", "serde_json"), |b| {
        let obj = serde_json_upstream::Value::Array(
            (0..1000)
                .map(|i| serde_json_upstream::Value::Number(i.into()))
                .collect(),
        );
        b.iter(|| serde_json_upstream::to_string(&obj));
    });

    group.bench_function(BenchmarkId::new("string_escapes", "lifegraph"), |b| {
        let mut obj = JsonValue::Object(Map::new());
        obj.push_field(
            "msg",
            "Hello \"World\"!\nLine\tbreak\rwith\tspecial\u{0001}chars",
        );
        b.iter(|| lg_to_string(&obj));
    });
    group.bench_function(BenchmarkId::new("string_escapes", "serde_json"), |b| {
        let mut obj = serde_json_upstream::Map::new();
        obj.insert(
            "msg".into(),
            serde_json_upstream::Value::String(
                "Hello \"World\"!\nLine\tbreak\rwith\tspecial\u{0001}chars".into(),
            ),
        );
        let obj = serde_json_upstream::Value::Object(obj);
        b.iter(|| serde_json_upstream::to_string(&obj));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 6. TYPED SERIALIZATION (to_string<&T: Serialize> — real-world pattern)
// ---------------------------------------------------------------------------

fn typed_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("typed_serialize");

    group.bench_function(BenchmarkId::new("small", "lifegraph"), |b| {
        let val = SmallPayload {
            id: 42,
            name: "test".into(),
            active: true,
        };
        b.iter(|| lg_to_string(&val));
    });
    group.bench_function(BenchmarkId::new("small", "serde_json"), |b| {
        let val = SmallPayload {
            id: 42,
            name: "test".into(),
            active: true,
        };
        b.iter(|| serde_json_upstream::to_string(&val));
    });

    let medium_lg = MediumPayload {
        user: User {
            id: 12345,
            name: "John Doe".into(),
            email: "john@example.com".into(),
            roles: vec!["admin".into(), "user".into(), "editor".into()],
            metadata: UserMeta {
                created: 1234567890,
                updated: 9876543210,
                tags: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
            },
        },
        status: "active".into(),
        count: 100,
    };
    group.bench_function(BenchmarkId::new("medium", "lifegraph"), |b| {
        b.iter(|| lg_to_string(&medium_lg));
    });
    group.bench_function(BenchmarkId::new("medium", "serde_json"), |b| {
        let val = MediumPayload {
            user: User {
                id: 12345,
                name: "John Doe".into(),
                email: "john@example.com".into(),
                roles: vec!["admin".into(), "user".into(), "editor".into()],
                metadata: UserMeta {
                    created: 1234567890,
                    updated: 9876543210,
                    tags: vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
                },
            },
            status: "active".into(),
            count: 100,
        };
        b.iter(|| serde_json_upstream::to_string(&val));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 7. BYTE PATHS (to_vec / from_slice — important for network/DB workloads)
// ---------------------------------------------------------------------------

fn byte_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("byte_paths");

    group.bench_function(BenchmarkId::new("to_vec", "lifegraph"), |b| {
        let obj = medium_lg_value();
        b.iter(|| lg_to_vec(&obj));
    });
    group.bench_function(BenchmarkId::new("to_vec", "serde_json"), |b| {
        let obj: serde_json_upstream::Value = serde_json_upstream::json!({
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
        });
        b.iter(|| serde_json_upstream::to_vec(&obj));
    });

    group.bench_function(BenchmarkId::new("from_slice", "lifegraph"), |b| {
        let json = medium_json();
        b.iter(|| lg_from_slice::<JsonValue>(json.as_bytes()));
    });
    group.bench_function(BenchmarkId::new("from_slice", "serde_json"), |b| {
        let json = medium_json();
        b.iter(|| serde_json_upstream::from_slice::<serde_json_upstream::Value>(json.as_bytes()));
    });

    group.bench_function(BenchmarkId::new("typed_from_slice", "lifegraph"), |b| {
        let json = medium_json();
        b.iter(|| lg_from_slice::<MediumPayload>(json.as_bytes()));
    });
    group.bench_function(BenchmarkId::new("typed_from_slice", "serde_json"), |b| {
        let json = medium_json();
        b.iter(|| serde_json_upstream::from_slice::<MediumPayload>(json.as_bytes()));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 8. ROUNDTRIP (parse → serialize — common in proxies/transformers)
// ---------------------------------------------------------------------------

fn roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    group.bench_function(BenchmarkId::new("small", "lifegraph"), |b| {
        let json = small_json();
        b.iter(|| {
            let val: JsonValue = parse_json(json).unwrap();
            lg_to_string(&val)
        });
    });
    group.bench_function(BenchmarkId::new("small", "serde_json"), |b| {
        let json = small_json();
        b.iter(|| {
            let val: serde_json_upstream::Value = serde_json_upstream::from_str(json).unwrap();
            serde_json_upstream::to_string(&val)
        });
    });

    group.bench_function(BenchmarkId::new("medium", "lifegraph"), |b| {
        let json = medium_json();
        b.iter(|| {
            let val: JsonValue = parse_json(json).unwrap();
            lg_to_string(&val)
        });
    });
    group.bench_function(BenchmarkId::new("medium", "serde_json"), |b| {
        let json = medium_json();
        b.iter(|| {
            let val: serde_json_upstream::Value = serde_json_upstream::from_str(json).unwrap();
            serde_json_upstream::to_string(&val)
        });
    });

    group.bench_function(BenchmarkId::new("large_array", "lifegraph"), |b| {
        let json = large_array_json();
        b.iter(|| {
            let val: JsonValue = parse_json(&json).unwrap();
            lg_to_string(&val)
        });
    });
    group.bench_function(BenchmarkId::new("large_array", "serde_json"), |b| {
        let json = large_array_json();
        b.iter(|| {
            let val: serde_json_upstream::Value = serde_json_upstream::from_str(&json).unwrap();
            serde_json_upstream::to_string(&val)
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion entry point
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    owned_parse,
    borrowed_parse,
    tape_parse,
    typed_deserialize,
    serialize,
    typed_serialize,
    byte_paths,
    roundtrip,
);
criterion_main!(benches);
