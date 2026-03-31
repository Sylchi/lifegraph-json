# Changelog

## 0.1.1 - 2026-04-01

Compatibility-focused follow-up release.

### Added
- `Value` and `Number` aliases
- `from_str`, `from_slice`, `to_string`, `to_vec`
- `Value` accessors (`as_str`, `as_bool`, `as_i64`, `as_u64`, `as_f64`)
- `Value` predicates (`is_null`, `is_array`, `is_object`, etc.)
- object lookup helpers (`get`, `get_mut`)
- indexing support (`value["field"]`, `value[index]`)
- lightweight `json!` macro

## 0.1.0 - 2026-04-01

Initial release.

### Included
- manual JSON serializer
- owned parser
- borrowed parser
- tape parser
- lazy hashed object indexing
- compiled lookup keys
- compiled object and row schema serialization

### Performance direction
- strong wins on tape parsing for structural/inspection-heavy workloads
- faster repeated lookup on wide objects with indexed compiled keys
- repeated-shape serialization fast paths
