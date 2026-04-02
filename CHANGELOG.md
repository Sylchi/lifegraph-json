# Changelog

## 0.1.16 - 2026-04-02

Full serde integration and test coverage release.

### Added
- Complete serde feature integration with `to_value`, `from_value`, `to_string`, `from_str`, etc.
- Feature-gated generic versions of all serialization/deserialization functions
- `Map::swap_remove`, `Map::shift_insert`, `Map::sort_keys` for upstream parity
- Upstream serde_json test suite (adapted `tests/serde_map_test.rs`)
- Comprehensive behavioral parity tests for RawValue and serialization
- Documentation for serde compatibility and test coverage

### Fixed
- Serde module declarations (`serde_deserialize`, `serde_serialize`, `serde_error`, `raw`)
- Import paths in serde modules to use `crate::` instead of `crate::value::`, etc.
- `JsonMapAccess` to properly track pending values for serde deserialization
- Test type annotations for serde feature compatibility

### Changed
- `serde` feature is now optional (not in default features)
- Default features: `["std"]` only, maintaining zero-dependency by default

### Testing
- 49 tests pass with `--features serde`
- All upstream serde_json map tests pass
- Full RawValue support (boxed deserialization)

## 0.1.15 - 2026-04-01

Benchmark-and-positioning release.

### Added
- normalized benchmark results against the `serde-rs/json-benchmark` corpus
- clearer README positioning around zero-dependency usage versus serde-driven workflows
- stronger migration guidance for `serde_json`-style `Value` usage

### Changed
- serde-compatible parse, serialize, and `Value` conversion paths are now implemented locally instead of delegating to upstream `serde_json`
- upstream `serde_json` is retained only as a dev/test parity oracle, not a runtime dependency

## 0.1.14 - 2026-04-01

Parser regression coverage release.

### Added
- more non-serde parser regression tests for malformed numbers, arrays, and objects
- additional compatibility coverage for parse-error edge cases

## 0.1.13 - 2026-04-01

Pure JSON parse compatibility release.

## 0.1.12 - 2026-04-01

Map ergonomics refinement release.

## 0.1.11 - 2026-04-01

Map parity release.

## 0.1.10 - 2026-04-01

Real serde_json example compatibility release.

## 0.1.9 - 2026-04-01

Number parity refinement release.

## 0.1.8 - 2026-04-01

`json!` macro parity upgrade.

## 0.1.7 - 2026-04-01

Primitive comparison parity release.

## 0.1.6 - 2026-04-01

Value parity refinement release.

## 0.1.5 - 2026-04-01

Compatibility API sweep.

## 0.1.4 - 2026-04-01

Drop-in compatibility push.

## 0.1.3 - 2026-04-01

Compatibility and ergonomics release.

## 0.1.2 - 2026-04-01

Compatibility expansion release.

## 0.1.1 - 2026-04-01

Compatibility-focused follow-up release.

## 0.1.0 - 2026-04-01

Initial release.
