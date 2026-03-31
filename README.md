# lifegraph-json

Zero-dependency JSON crate in Rust with **owned**, **borrowed**, **tape**, and **compiled-schema** paths.

## Why

`lifegraph-json` is aimed at workloads where generic JSON trees leave performance on the table:

- fast parse-and-inspect flows
- low-allocation parsing
- repeated lookup on wide objects
- repeated serialization of known object shapes

It uses **0 runtime dependencies**.

## Compatibility surface

`lifegraph-json` now includes a growing compatibility-oriented API modeled after common `serde_json` usage:

- `Value`, `Number`, `Map`
- `from_str`, `from_slice`, `from_reader`
- `to_string`, `to_vec`, `to_writer`
- `json!`
- `value["field"]` and `value[index]`
- generic `get`, `get_mut`, plus `get_index`, `get_index_mut`
- `as_str`, `as_bool`, `as_i64`, `as_u64`, `as_f64`
- `is_null`, `is_array`, `is_object`, `len`, `is_empty`, `sort_all_objects`

It is **not** fully drop-in compatible with `serde_json` yet, but simple code ports are getting much easier.

Newer compatibility helpers now include JSON Pointer access (`Value::pointer`, `Value::pointer_mut`), `Value::take`, pretty-print output helpers, number/null parity helpers, and nested mutable indexing like `value["a"]["b"] = ...`.

## Migration sketch

```rust
// before
// use serde_json::{json, Value};

// after
use lifegraph_json::{json, from_str, to_string, Value};

let value: Value = from_str(r#"{"ok":true,"n":7}"#)?;
assert_eq!(value["ok"].as_bool(), Some(true));
assert_eq!(value["n"].as_i64(), Some(7));

let built = json!({"msg": "hello", "items": [1, 2, null]});
let encoded = to_string(&built)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Example: reader/writer helpers

```rust
use lifegraph_json::{from_reader, to_writer};
use std::io::Cursor;

let value = from_reader(Cursor::new(br#"{"a":1,"b":[true,false]}"# as &[u8]))?;
let mut out = Vec::new();
to_writer(&mut out, &value)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

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

## Performance direction

On local release-mode comparisons against `serde_json`, the strongest wins so far have been in specialized paths such as:

- tape parsing on medium/report-like payloads
- deep structural parses
- wide-object repeated lookup with indexed compiled keys

Best observed outliers so far include roughly:

- **up to ~6x faster** on deep structural parses
- **~4x faster** on several tape parse / parse+lookup workloads
- **~3x faster** on indexed repeated lookup over wide objects

This crate is best viewed as a **performance-oriented JSON toolkit for specific workloads**, with a growing compatibility layer for easier adoption.
