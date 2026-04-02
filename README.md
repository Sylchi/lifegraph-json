# lifegraph-json

**A small JSON crate with zero runtime dependencies by default that can beat `serde_json` by up to 3.78x on the `serde-rs/json-benchmark` corpus, and by ~6x on some structural parse workloads.**

`lifegraph-json` is a fast JSON value layer for Rust with **owned**, **borrowed**, **tape**, and **compiled-schema** paths.

## Why this exists

`serde_json` is fantastic infrastructure, but it optimizes for broad ecosystem integration and typed serde workflows.

`lifegraph-json` optimizes for a different target:

- **0 runtime dependencies by default**
- **fast parse-and-inspect flows**
- **low-allocation parsing**
- **wide-object lookup**
- **repeated serialization of known object shapes**
- a compatibility-focused `Value`-style API for easier swapping

If you want a small, fast, hackable JSON layer with aggressive specialized paths, this is the crate.

## Important compatibility note

`lifegraph-json` intentionally does **not** depend on `serde` by default.

When the `serde` feature is enabled, the crate provides its own local implementations for:

- `from_str::<T>`
- `from_slice::<T>` / `from_reader::<T>`
- `Serialize` / `Deserialize`
- `to_value` / `from_value`
- `to_string`, `to_vec`, `to_writer`, and pretty variants for typed `T`

That means the main serde-compatible API surface is available without delegating back to upstream `serde_json`.

What is still intentionally out of scope is the broader `serde_json` ecosystem surface beyond these APIs. If you need obscure upstream-specific behavior or exact implementation parity in edge cases, evaluate those cases directly.

The crate keeps an upstream `serde_json` copy only in dev/test dependencies as a parity oracle. Runtime parse and serialize behavior is implemented locally.

If you want a **fast zero-runtime-dependency-by-default JSON layer with a local serde-compatible path**, use `lifegraph-json`.

## What feels drop-in already

`lifegraph-json` includes a growing compatibility-oriented API modeled after common `serde_json` usage:

- `Value`, `Number`, `Map`
- `from_str`, `from_slice`, `from_reader`
- `to_string`, `to_vec`, `to_writer`
- `to_string_pretty`, `to_vec_pretty`, `to_writer_pretty`
- `json!`
- `value["field"]` and `value[index]`
- generic `get`, `get_mut`, plus `get_index`, `get_index_mut`
- `pointer`, `pointer_mut`, `take`
- `as_str`, `as_bool`, `as_i64`, `as_u64`, `as_f64`
- `is_null`, `is_array`, `is_object`, `len`, `is_empty`, `sort_all_objects`
- nested mutable indexing like `value["a"]["b"] = ...`
- primitive comparisons like `assert_eq!(value["ok"], true)`

In practice this now covers most common `Value`-centric `serde_json` code paths, plus the main typed serde entry points.

## Quick migration sketch

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

## Normalized benchmark snapshot

Benchmarked locally in release mode against the official [`serde-rs/json-benchmark`](https://github.com/serde-rs/json-benchmark) data corpus (`canada.json`, `citm_catalog.json`, `twitter.json`, commit `17b13dd`).

To normalize out CPU differences, the main takeaway here is the **ratio** versus `serde_json`, not the raw MB/s.

### Best observed ratios on this machine

- **tape parse:** up to **3.78x faster**
- **borrowed parse:** up to **2.37x faster**
- **owned parse:** up to **1.35x faster**
- **DOM stringify:** up to **1.12x faster** on this corpus

### Geometric-mean ratios across the three benchmark files

- **owned parse:** `lifegraph-json` **1.17x** `serde_json`
- **borrowed parse:** `lifegraph-json` **1.55x** `serde_json`
- **tape parse:** `lifegraph-json` **2.93x** `serde_json`
- **DOM stringify:** `serde_json` **1.42x** `lifegraph-json`

### Per-file snapshot

| Corpus | Owned parse | Borrowed parse | Tape parse | DOM stringify |
|---|---:|---:|---:|---:|
| `canada.json` | `serde_json` 1.13x | `serde_json` 1.11x | `lifegraph-json` 2.39x | `serde_json` 3.27x |
| `citm_catalog.json` | `lifegraph-json` 1.33x | `lifegraph-json` 1.76x | `lifegraph-json` 2.79x | `lifegraph-json` 1.12x |
| `twitter.json` | `lifegraph-json` 1.35x | `lifegraph-json` 2.37x | `lifegraph-json` 3.78x | `lifegraph-json` 1.03x |

So the honest story is:

- `lifegraph-json` is **not faster everywhere**
- its **tape** and **borrowed** paths are where the strongest wins live
- stringify is getting better, but `serde_json` still wins overall there
- if your workload is parse-heavy or parse-and-inspect heavy, `lifegraph-json` gets very interesting

## Performance direction beyond the benchmark corpus

Outside the `json-benchmark` corpus, local specialized benchmarks have also shown larger outliers on structural-heavy workloads, including roughly:

- **~4x faster** on several tape parse / parse+lookup workloads
- **~3x faster** on indexed repeated lookup over wide objects
- **up to ~6x faster** on some deep structural parse cases

This crate is best viewed as a **performance-oriented JSON toolkit with a local `serde_json`-style compatibility layer**.

## Reader/writer example

```rust
use lifegraph_json::{from_reader, to_writer};
use std::io::Cursor;

let value = from_reader(Cursor::new(br#"{"a":1,"b":[true,false]}"# as &[u8]))?;
let mut out = Vec::new();
to_writer(&mut out, &value)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Tape parsing example

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

## `json!` macro parity

The macro is much closer to `serde_json` in practice, including expression-key object entries:

```rust
# use lifegraph_json::json;
let code = 200;
let features = vec!["serde", "json"];
let value = json!({
    "code": code,
    "success": code == 200,
    features[0]: features[1],
});
assert_eq!(value["serde"], "json");
```
