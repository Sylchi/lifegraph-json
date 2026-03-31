# lifegraph-json

Dependency-light JSON toolkit in Rust with **owned**, **borrowed**, **tape**, and **compiled-schema** paths.

## Why

`lifegraph-json` is aimed at workloads where generic JSON trees leave performance on the table:

- fast parse-and-inspect flows
- low-allocation parsing
- repeated lookup on wide objects
- repeated serialization of known object shapes

It uses no external runtime dependencies.

## Features

- manual JSON serializer
- owned parser
- borrowed parser
- tape parser for fast structural access
- lazy hashed object indexing
- compiled lookup keys for repeated field queries
- compiled object and row schemas for repeated-shape serialization

## Example: tape parsing with compiled lookup keys

```rust
use lifegraph_json::{parse_json_tape, CompiledTapeKeys, TapeTokenKind};

let input = r#"{"name":"hello","flag":true}"#;
let tape = parse_json_tape(input)?;
let root = tape.root(input).unwrap();
let index = root.build_object_index().unwrap();
let indexed = root.with_index(&index);
let keys = CompiledTapeKeys::new(&["name", "flag"]);
let kinds = indexed
    .get_compiled_many(&keys)
    .map(|value| value.unwrap().kind())
    .collect::<Vec<_>>();

assert_eq!(kinds, vec![TapeTokenKind::String, TapeTokenKind::Bool]);
# Ok::<(), lifegraph_json::JsonParseError>(())
```

## Example: compiled row serialization

```rust
use lifegraph_json::{CompiledRowSchema, JsonValue};

let schema = CompiledRowSchema::new(&["id", "name"]);
let row1 = [JsonValue::from(1u64), JsonValue::from("a")];
let row2 = [JsonValue::from(2u64), JsonValue::from("b")];

let json = schema.to_json_string([row1.iter(), row2.iter()])?;
assert_eq!(json, r#"[{"id":1,"name":"a"},{"id":2,"name":"b"}]"#);
# Ok::<(), lifegraph_json::JsonError>(())
```

## Current performance direction

On local release-mode comparisons against `serde_json`, the strongest wins so far have been in specialized paths such as:

- tape parsing on medium/report-like payloads
- deep structural parses
- wide-object repeated lookup with indexed compiled keys

This crate is best viewed as a **performance-oriented JSON toolkit for specific workloads**, not a blanket replacement for `serde_json`.

## Status

Current focus:

- correctness
- benchmarked fast parse and lookup paths
- zero/low-allocation access modes
- repeated-shape serialization
