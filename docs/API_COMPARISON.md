# serde_json vs lifegraph-json API Compatibility

## Top-Level Functions

| serde_json                    | lifegraph-json          | Return (serde_json) | Return (lifegraph-json)        | Match? |
|-------------------------------|-------------------------|---------------------|--------------------------------|--------|
| `from_str`                    | `from_str`              | `Result<T, Error>`  | `Result<T, Error>` (serde)     | ✅     |
|                               |                         |                     | `Result<JsonValue, JsonParseError>` (no serde) | ⚠️  |
| `from_slice`                  | `from_slice`            | `Result<T, Error>`  | `Result<T, Error>` (serde)     | ✅     |
|                               |                         |                     | `Result<JsonValue, JsonParseError>` (no serde) | ⚠️  |
| `from_reader`                 | `from_reader`           | `Result<T, Error>`  | `Result<T, Error>` (serde)     | ✅     |
|                               |                         |                     | `Result<JsonValue, JsonParseError>` (no serde) | ⚠️  |
| `from_value`                  | `from_value`            | `Result<T, Error>`  | `Result<T, Error>`             | ✅     |
| `to_string`                   | `to_string`             | `Result<String, E>` | `Result<String, Error>` (serde)| ✅     |
|                               |                         |                     | `Result<String, JsonError>` (no serde) | ⚠️  |
| `to_vec`                      | `to_vec`                | `Result<Vec<u8>, E>`| `Result<Vec<u8>, Error>` (serde)| ✅    |
|                               |                         |                     | `Result<Vec<u8>, JsonError>` (no serde) | ⚠️  |
| `to_writer`                   | `to_writer`             | `Result<(), Error>` | `Result<(), Error>` (serde)    | ✅     |
|                               |                         |                     | `Result<(), JsonError>` (no serde) | ⚠️   |
| `to_string_pretty`            | `to_string_pretty`      | `Result<String, E>` | `Result<String, Error>` (serde)| ✅     |
| `to_vec_pretty`               | `to_vec_pretty`         | `Result<Vec<u8>, E>`| `Result<Vec<u8>, Error>` (serde)| ✅    |
| `to_writer_pretty`            | `to_writer_pretty`      | `Result<(), Error>` | `Result<(), Error>` (serde)    | ✅     |
| `to_value`                    | `to_value`              | `Result<Value, E>`  | `Result<JsonValue, Error>`     | ✅     |
| `value_raw`                   | *(missing)*             | `Result<String, E>` | —                              | ❌     |

## Types

| serde_json      | lifegraph-json  | serde_json type              | lifegraph-json type            | Match? |
|-----------------|-----------------|------------------------------|--------------------------------|--------|
| `Value`         | `Value` / `JsonValue` | `enum Value` (6 variants)   | `enum JsonValue` (6 variants)  | ✅     |
| `Number`        | `Number` / `JsonNumber` | `struct Number` (opaque)     | `enum JsonNumber {I64,U64,F64}` | ⚠️    |
| `Map<K,V>`      | `Map`           | `Map<String, Value>` (generic) | `Map = Vec<(String, JsonValue)>` | ⚠️  |
| `Error`         | `Error`         | `struct Error`               | `struct Error`                 | ✅     |
| `Result<T>`     | *(missing)*     | `type Result<T> = Result<T,Error>` | —                         | ❌     |

## Additional Types (ours, not in serde_json)

| Type                | Purpose                                  |
|---------------------|------------------------------------------|
| `BorrowedJsonValue<'a>` | Zero-copy borrowed JSON value        |
| `JsonTape`          | Tokenized JSON for fast random access    |
| `TapeToken`         | Single token in a tape                   |
| `TapeTokenKind`     | Token type (Null, Bool, Number, etc.)    |
| `TapeValue<'a>`     | View into a tape at a specific index     |
| `TapeObjectIndex`   | Hash index for fast object key lookup    |
| `IndexedTapeObject` | Tape + index combined for fast access    |
| `CompiledObjectSchema` | Pre-compiled schema for fast object serialization |
| `CompiledRowSchema` | Pre-compiled schema for array-of-objects |
| `CompiledTapeKey`   | Pre-hashed key for tape lookup           |
| `CompiledTapeKeys`  | Collection of pre-hashed keys            |
| `ValueIndex`        | Trait for generic indexing (`value["key"]`) |
| `RawValue`          | Opaque raw JSON text (zero-copy)         |

## Functions (ours, not in serde_json)

| Function              | Return                                | Purpose                    |
|-----------------------|---------------------------------------|----------------------------|
| `parse_json`          | `Result<JsonValue, JsonParseError>`   | Pure JSON → JsonValue      |
| `parse_json_borrowed` | `Result<BorrowedJsonValue, JsonParseError>` | Zero-copy parsing   |
| `parse_json_tape`     | `Result<JsonTape, JsonParseError>`    | Tokenize JSON into tape    |
| `escape_json_string`  | `String`                              | Escape a string for JSON   |

## Macros

| Macro   | serde_json | lifegraph-json | Match? |
|---------|------------|----------------|--------|
| `json!` | ✅          | ✅              | ✅     |

## Missing from lifegraph-json

| Item                  | Purpose                                    |
|-----------------------|--------------------------------------------|
| `Result<T>` type alias | Convenience alias `Result<T, Error>`       |
| `StreamDeserializer`  | Iterator over consecutive JSON values       |
| `Deserializer`        | Low-level streaming deserializer            |
| `Serializer`          | Low-level streaming serializer              |
| `ser` module          | Serialization utilities                     |
| `de` module           | Deserialization utilities                   |
| `value_raw`           | Serialize to raw JSON string                |

## Key Differences

### 1. `from_str` return type divergence
- serde_json: always `Result<T, Error>` where T: DeserializeOwned
- lifegraph-json (no serde): `Result<JsonValue, JsonParseError>` — different error type

### 2. `Number` representation
- serde_json: opaque struct with getter methods
- lifegraph-json: public enum `I64(i64) | U64(u64) | F64(f64)` — exposes internals

### 3. `Map` not generic
- serde_json: `Map<K, V>` (defaults to `Map<String, Value>`)
- lifegraph-json: `Map` is always `Vec<(String, JsonValue)>` — no key/value generics

### 4. Error type differs without serde
- serde_json: always `serde_json::Error`
- lifegraph-json (no serde): `JsonParseError` (different type, different variants)
