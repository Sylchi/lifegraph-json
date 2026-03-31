# Changelog

## 0.1.2 - 2026-04-01

Compatibility expansion release.

### Added
- `Map` alias
- `from_reader`
- `to_writer`
- `len` and `is_empty`
- `get_index` and `get_index_mut`
- `as_i128`, `as_u128`, `as_f32`
- `FromIterator` support for arrays and objects
- migration-oriented README examples

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
