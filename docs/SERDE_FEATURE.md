# Serde Feature: What Changes

## Overview

The `serde` feature controls whether lifegraph-json integrates with the [serde](https://serde.rs/) ecosystem.
This changes both the **public API** and the **performance characteristics**.

For a detailed breakdown of what works, what doesn't, and known limitations:

📖 **[See COMPATIBILITY_GUIDE.md](COMPATIBILITY_GUIDE.md)**

## Without `serde` (default, fast path)

```toml
[dependencies]
lifegraph-json = "1.0"
# zero extra dependencies
```

| Aspect | Behavior |
|--------|----------|
| Dependencies | **None** (pure Rust) |
| `from_str(input)` | → `Result<JsonValue, JsonParseError>` (direct parser call) |
| `to_string(&value)` | → `Result<String, JsonError>` |
| Error type | `JsonParseError` (has `InvalidUtf8`, `UnexpectedEnd`, etc.) |
| Performance | **Maximum** — direct `Parser::parse_value()` → `JsonValue` |

Use this when you only need `JsonValue` tree manipulation without serde types.

## With `serde` (compatible with serde_json)

```toml
[dependencies]
lifegraph-json = { version = "1.0", features = ["serde"] }
```

| Aspect | Behavior |
|--------|----------|
| Dependencies | + `serde` (+ `serde_derive` at compile time) |
| `from_str::<T>(input)` | → `Result<T, Error>` where `T: DeserializeOwned` (serde dispatch) |
| `to_string(&value)` | → `Result<String, Error>` (serde path for non-JsonValue types) |
| Error type | `serde_error::Error` (has `line()`, `column()`, `category()`) |
| Performance | **Slower for `from_str`** — goes through serde visitor pattern |
| Compatibility | Drop-in replacement for `serde_json` API |

## ⚠️ Performance Warning

When `serde` is enabled, **do not use `from_str` to get `JsonValue`**. It routes through
serde's `Deserialize` visitor machinery, adding dispatch overhead for every JSON node.

Instead, use `parse_json` for raw JSON → `JsonValue` parsing — it's fast in both modes:

```rust
use lifegraph_json::{parse_json, from_str, JsonValue};

// ✅ Fast path — always calls Parser::parse_value() directly
let value: JsonValue = parse_json(input)?;

// ⚠️ Slow when serde feature is on — goes through serde Deserialize
let value: JsonValue = from_str(input)?;

// ✅ Correct: use from_str when deserializing INTO your own types
#[derive(serde::Deserialize)]
struct MyStruct { name: String }
let my_struct: MyStruct = from_str(input)?;
```

## Function Behavior by Feature

| Function          | `serde` OFF                           | `serde` ON                              |
|-------------------|---------------------------------------|-----------------------------------------|
| `parse_json`      | `Result<JsonValue, JsonParseError>`   | Same — **always fast**                  |
| `from_str`        | `Result<JsonValue, JsonParseError>`   | `Result<T, Error>` (serde dispatch)     |
| `from_slice`      | `Result<JsonValue, JsonParseError>`   | `Result<T, Error>` (serde dispatch)     |
| `from_reader`     | `Result<JsonValue, JsonParseError>`   | `Result<T, Error>` (serde dispatch)     |
| `from_value`      | ❌ Not available                      | `Result<T, Error>`                      |
| `to_value`        | ❌ Not available                      | `Result<JsonValue, Error>`              |
| `to_string`       | `Result<String, JsonError>`           | `Result<String, Error>` (serde path)    |
| `to_vec`          | `Result<Vec<u8>, JsonError>`          | `Result<Vec<u8>, Error>` (serde path)   |

## Choosing the Right Feature

| Scenario | Feature |
|----------|---------|
| Parse JSON, get `JsonValue`, manipulate tree | No `serde` (default) |
| `#[derive(Deserialize, Serialize)]` on your structs | `serde` |
| Drop-in replacement for `serde_json` | `serde` |
| Zero-dependency binary | No `serde` |
| Performance-critical JSON parsing | No `serde` + use `parse_json()` |
