# lifegraph-json

[![CI](https://github.com/Sylchi/lifegraph-json/actions/workflows/ci.yml/badge.svg)](https://github.com/Sylchi/lifegraph-json/actions/workflows/ci.yml)
[![Miri](https://github.com/Sylchi/lifegraph-json/actions/workflows/miri.yml/badge.svg)](https://github.com/Sylchi/lifegraph-json/actions/workflows/miri.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/lifegraph-json.svg)](https://crates.io/crates/lifegraph-json)
[![Documentation](https://docs.rs/lifegraph-json/badge.svg)](https://docs.rs/lifegraph-json)

**A small JSON crate with zero runtime dependencies by default that can beat `serde_json` by up to 3.78x on the `serde-rs/json-benchmark` corpus, and by ~6x on some structural parse workloads.**

`lifegraph-json` is a fast JSON value layer for Rust with **owned**, **borrowed**, **tape**, and **compiled-schema** paths.

## Quick Start

```toml
[dependencies]
lifegraph-json = "1.0"
```

```rust
use lifegraph_json::{json, from_str, to_string, Value};

let value: Value = from_str(r#"{"ok":true,"n":7}"#)?;
assert_eq!(value["ok"].as_bool(), Some(true));
assert_eq!(value["n"].as_i64(), Some(7));

let built = json!({"msg": "hello", "items": [1, 2, null]});
let encoded = to_string(&built)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Why This Exists

`serde_json` is fantastic infrastructure, but it optimizes for broad ecosystem integration and typed serde workflows.

`lifegraph-json` optimizes for a different target:

- **0 runtime dependencies by default**
- **fast parse-and-inspect flows**
- **low-allocation parsing**
- **wide-object lookup**
- **repeated serialization of known object shapes**
- a compatibility-focused `Value`-style API for easier swapping

If you want a small, fast, hackable JSON layer with aggressive specialized paths, this is the crate.

| Feature | `lifegraph-json` | `serde_json` |
|---------|-----------------|--------------|
| Runtime dependencies | **0** (by default) | serde (1+) |
| Parse speed | **Up to 3.78× faster** (tape path) | Baseline |
| Tape parsing | ✅ Yes | ❌ No |
| Compiled schemas | ✅ Yes | ❌ No |
| Serde integration | Optional | Built-in |
| `from_str::<T>` | Via `serde` feature | Native |

**Use `lifegraph-json` when you want:**
- Maximum parse performance with zero dependencies
- Fast parse-and-inspect flows without full deserialization
- Low-allocation parsing with borrowed/tape paths
- Repeated serialization of known object shapes

**Use `serde_json` when you need:**
- Full serde ecosystem integration
- Typed deserialization without optional features
- Maximum compatibility with existing code

## Important compatibility note

`lifegraph-json` intentionally does **not** depend on `serde` by default.

When the `serde` feature is enabled, the crate provides its own local implementations for:

- `from_str::<T>`
- `from_slice::<T>` / `from_reader::<T>`
- `Serialize` / `Deserialize`
- `to_value` / `from_value`
- `to_string`, `to_vec`, `to_writer`, and pretty variants for typed `T`

That means the main serde-compatible API surface is available without delegating back to upstream `serde_json`.

### What works and what doesn't?

For a comprehensive breakdown of compatibility, known differences, and limitations:

📖 **[See COMPATIBILITY_GUIDE.md](docs/COMPATIBILITY_GUIDE.md)**

**Quick summary**:
- ✅ **90%+ of serde_json use cases**: Drop-in replacement with `serde` feature
- ✅ Same API surface for common patterns
- ✅ Better performance (up to 3.78x faster parsing)
- ⚠️ Minor differences: float precision edge cases, key ordering (always preserved)
- ❌ Missing: Low-level streaming APIs, some advanced serde features

What is still intentionally out of scope is the broader `serde_json` ecosystem surface beyond these APIs. If you need obscure upstream-specific behavior or exact implementation parity in edge cases, evaluate those cases directly.

The crate keeps an upstream `serde_json` copy only in dev/test dependencies as a parity oracle. Runtime parse and serialize behavior is implemented locally.

If you want a **fast zero-runtime-dependency-by-default JSON layer with a local serde-compatible path**, use `lifegraph-json`.

## Pros and Cons vs serde_json

### When lifegraph-json Excels

**🚀 Performance-critical parse-and-inspect workloads**
- Tape parsing is up to **3.78× faster** than serde_json on the json-benchmark corpus
- Borrowed parsing avoids String allocations for string values and keys
- Ideal when you parse JSON, read a few fields, and discard

**📦 Zero runtime dependencies**
- No serde dependency unless you enable the `serde` feature
- Smaller compile times, simpler dependency tree
- Useful for projects minimizing external dependencies

**🔍 Field access on wide objects**
- Optimized lookup for objects with many keys
- Tape indexed lookup for repeated field access patterns
- Better performance when reading specific fields from large JSON

**⚡ Specialized parsing paths**
- Owned parsing: standard full-copy approach
- Borrowed parsing: zero-copy for strings/keys (lifetime-bound)
- Tape parsing: fast token stream with indexed lookups
- Choose the right path for your workload

**🔄 Repeated serialization**
- Efficient for serializing the same structure many times
- Known object shapes serialize faster
- Good for generating JSON templates or responses

### When serde_json is Better

**🌐 Ecosystem integration**
- Works seamlessly with serde-derived types across the ecosystem
- Most Rust libraries expect serde_json::Value
- Established compatibility with ORMs, web frameworks, etc.

**📝 Typed deserialization out-of-the-box**
- No feature flags needed for `from_str::<T>`
- Derive macros work immediately
- No configuration required

**🔧 Advanced serde features**
- `#[serde(rename)]`, `#[serde(default)]`, `#[serde(skip)]` attributes
- Custom serialization via `Serialize`/`Deserialize` impls
- Flatten, tag, and other complex serde patterns
- Untagged enums and complex type mappings

**📊 Streaming APIs**
- `Deserializer::from_reader` for large files
- Incremental parsing for memory-constrained environments
- Stream processing of JSON arrays/objects

**🎯 Exact precision and edge cases**
- Battle-tested float handling
- Precise control over number formatting
- Years of edge-case resolutions in production

**📖 Documentation and community**
- Extensive docs, tutorials, and Stack Overflow answers
- Large community for troubleshooting
- Proven in production at scale across thousands of projects

### Decision Guide

| Your Use Case | Recommendation |
|---------------|----------------|
| Parse JSON, read 2-3 fields, discard | **lifegraph-json** (tape/borrowed) |
| Zero dependencies required | **lifegraph-json** |
| Full serde ecosystem types | **serde_json** |
| Complex `#[serde(...)]` attributes | **serde_json** |
| Stream large files from disk/network | **serde_json** |
| Serialize same structure repeatedly | **lifegraph-json** |
| Simple Value-based JSON handling | **Both work** (lifegraph faster) |
| Production-critical edge cases | **serde_json** (more battle-tested) |
| Maximum parse speed needed | **lifegraph-json** |

## Performance

Benchmarked against the [`serde-rs/json-benchmark`](https://github.com/serde-rs/json-benchmark) corpus:

| Path | Geometric Mean | Best Case |
|------|---------------|-----------|
| Owned parse | **1.17×** faster | 1.35× (`twitter.json`) |
| Borrowed parse | **1.55×** faster | 2.37× (`twitter.json`) |
| Tape parse | **2.93×** faster | **3.78×** (`twitter.json`) |
| DOM stringify | 1.42× slower | 1.12× faster |

**Important notes about these measurements:**

- **Tape parse** is a lifegraph-json-specific feature that creates a token stream. serde_json has no equivalent, so this measures lifegraph against itself on different workloads. The "3.78× faster" claim refers to tape parsing vs serde_json's Value parsing on `twitter.json`.
- **Owned/borrowed parse** compares `parse_json()` → `JsonValue` vs `serde_json::from_str()` → `Value` (equivalent operations)
- **DOM stringify** measures `JsonValue::to_string()` vs `serde_json::Value::to_string()` (equivalent operations)
- Benchmarks now use **symmetric fixture construction** (both libraries construct test data identically, not via macros)
- All benchmarks report **bytes processed** for throughput measurement (MB/s)

**When to care about which metric:**
- Parse-and-inspect: Tape/borrowed paths are fastest
- Full deserialization: Owned parse is competitive
- Serialization: serde_json is generally faster for DOM stringify
- Typed serde: Both perform similarly (depends on your types)

See [benches/](benches/) for detailed benchmarks. Run with `cargo bench --features serde_json_bench`.

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

## API Modes

`lifegraph-json` provides multiple parsing paths for different use cases:

### Owned Path (Standard)
```rust
use lifegraph_json::{from_str, Value};
let value: Value = from_str(json)?;  // Full ownership, easy to use
```

### Borrowed Path (Zero-Copy Strings)
```rust
use lifegraph_json::parse_json_borrowed;
let value = parse_json_borrowed(json)?;  // Borrows from input, no String allocations
```

### Tape Path (Fast Parse + Inspect)
```rust
use lifegraph_json::parse_json_tape;
let tape = parse_json_tape(json)?;  // Structural index, fastest parse
let root = tape.root(json)?;
let index = root.build_object_index()?;
```

### Compiled Schema (Repeated Shapes)
```rust
use lifegraph_json::CompiledObjectSchema;
let schema = CompiledObjectSchema::new(&["id", "name", "active"]);
let json = schema.to_json_string(&[value1, value2, value3])?;
```

## Examples

### Reader/Writer
```rust
use lifegraph_json::{from_reader, to_writer};
use std::io::Cursor;

let value = from_reader(Cursor::new(br#"{"a":1,"b":[true,false]}"# as &[u8]))?;
let mut out = Vec::new();
to_writer(&mut out, &value)?;
```

### Tape Parsing with Compiled Keys
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
    .map(|v| v.unwrap().kind())
    .collect::<Vec<_>>();

assert_eq!(kinds, vec![TapeTokenKind::String, TapeTokenKind::Bool]);
```

### JSON Macro
```rust
use lifegraph_json::json;

let code = 200;
let features = vec!["serde", "json"];
let value = json!({
    "code": code,
    "success": code == 200,
    features[0]: features[1],
});
assert_eq!(value["serde"], "json");
```

## Features

| Feature | Description |
|---------|-------------|
| `std` (default) | Standard library support |
| `alloc` | Allocator support (no std) |
| `serde` | Serde Serialize/Deserialize support |
| `indexmap` | Use indexmap for ordered maps |
| `preserve_order` | Alias for indexmap |
| `raw_value` | Raw value support (requires serde) |

## Serde Compatibility

When the `serde` feature is enabled, `lifegraph-json` provides full serde integration:

```toml
[dependencies]
lifegraph-json = { version = "1.0", features = ["serde"] }
```

```rust
use lifegraph_json::{from_str, to_string, from_value, to_value, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Payload {
    ok: bool,
    count: u64,
    tags: Vec<String>,
}

// Typed deserialization
let payload: Payload = from_str(r#"{"ok":true,"count":7,"tags":["a","b"]}"#)?;

// Value-based serde
let value: Value = to_value(&payload)?;
let reparsed: Payload = from_value(value)?;
```

**Test Coverage:** The serde integration is tested against:
- Upstream serde_json test suite (adapted tests)
- Type-safe serialization/deserialization
- Error handling parity
- Raw value support (boxed `Box<RawValue>`)

See `tests/behavioral_parity.rs` and `tests/serde_map_test.rs` for compatibility tests.

## Safety & Testing

This crate takes correctness and safety seriously:

### Validation Pipeline
- **CI**: All tests run on Ubuntu, Windows, and macOS
- **Miri**: Memory safety validation on every PR
- **Fuzzing**: libFuzzer-based fuzzing for edge cases
- **JSONTestSuite**: Full compatibility test suite
- **Serde Compatibility**: Tests adapted from upstream serde_json

### Running Tests
```bash
# Run all tests (default features)
cargo test

# Run tests with serde feature
cargo test --features serde

# Run tests with all features
cargo test --all-features

# Run Miri (memory safety)
./scripts/miri.sh

# Run fuzzing
./scripts/fuzz.sh

# Run benchmarks
cargo bench
```

### Test Coverage
- **34 lib tests** - Core JSON parsing, serialization, and value operations
- **7 behavioral parity tests** - RawValue, serialization, error handling
- **3 serde map tests** - Adapted from upstream serde_json
- **9 integration tests** - Typed serde serialization/deserialization

See [docs/MIRI.md](docs/MIRI.md) and [docs/FUZZING.md](docs/FUZZING.md) for details.

## Development

### Setup
```bash
# Clone and set up
git clone https://github.com/Sylchi/lifegraph-json
cd lifegraph-json
./scripts/setup.sh
```

### Git Hooks
This project uses git hooks for quality assurance:
- **pre-commit**: Format check, clippy, fast tests
- **pre-push**: Full test suite, release build

Install with: `git config core.hooksPath .githooks`

### Making a Release
```bash
./scripts/release.sh 1.0.150
```

This validates, updates versions, creates a tag, and triggers CI publication.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contribution

Contributions are welcome! Please:
1. Run `./scripts/setup.sh` to configure hooks
2. Ensure all tests pass: `cargo test`
3. Run Miri: `./scripts/miri.sh`
4. Submit a PR

By contributing, you agree that your work will be dual-licensed under MIT and Apache-2.0.
