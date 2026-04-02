# serde_json 1.0.149 Public API Parity Report

Compared upstream `serde_json 1.0.149` source in the local cargo registry against `/tmp/lifegraph-json`.

Method:
- Source inventory from upstream `lib.rs`, `de.rs`, `ser.rs`, `error.rs`, `map.rs`, `number.rs`, `raw.rs`, and `value/mod.rs`
- Compile probes against the local crate with features `serde`, `raw_value`, and `preserve_order`
- Targeted method probes on exported types

This report only covers public API surface, not full semantic parity.

## Present Import Surface

These imports compile against the local crate:
- `serde_json::de::{from_reader, from_slice, from_str, Deserializer, StreamDeserializer}`
- `serde_json::ser::{to_writer, to_writer_pretty, to_vec, to_vec_pretty, to_string, to_string_pretty, Serializer, Formatter, CompactFormatter, PrettyFormatter}`
- `serde_json::error::{Error, Result, Category}`
- `serde_json::map::{Map, Entry, VacantEntry, OccupiedEntry}`
- `serde_json::value::{Index, Map, Number, RawValue, Value, to_value, from_value}`
- `serde_json::{Deserializer, StreamDeserializer, Serializer, Map, Number, Value}`

## Missing Import Surface

No remaining missing public imports were detected by the compile probes in this report.

## Missing Method-Level Surface On Existing Public Types

No remaining missing method-level items were detected by the targeted compile probes in this report.

## Method-Level Surface Confirmed Present

These targeted probes compile:
- `serde_json::de::{Read, SliceRead, StrRead, IoRead}`
- `serde_json::de::Deserializer::disable_recursion_limit()`
- `serde_json::ser::{State, Compound, CharEscape}`
- `serde_json::Map::clear()`
- `serde_json::Map::swap_remove(...)`
- `serde_json::Map::swap_remove_entry(...)`
- `serde_json::Map::shift_remove_entry(...)`
- `serde_json::Map::into_values()`
- `serde_json::Map::entry(...).or_default()`
- `serde_json::Number::from_string_unchecked(...)`
- `serde_json::value::RawValue::from_string(...)`
- `serde_json::value::to_raw_value(...)`
- `Into<std::io::Error>` for `serde_json::Error`

## Likely Structural Causes

- Public symbol parity is now substantially covered; the remaining risk is behavior, trait impl completeness, and feature semantics.

## Not Yet Proven By This Report

These still need deeper behavioral or trait-level auditing before any "100% compatible" claim:
- Exact parser behavior and error messages
- Error line and column semantics on all failure modes
- `RawValue` semantics and borrow/own behavior
  Top-level, nested, and enum-backed borrowed/boxed raw-value deserialization now match upstream in comparison tests.
- `preserve_order` behavior matching upstream `indexmap` mode
  Parse order and key-order mutation behavior are now covered by direct upstream comparison tests.
- Exact iterator trait impl parity
- Exact serializer formatting behavior
  Compact and pretty serializer output are now covered against upstream on a representative value corpus, but not exhaustively.
