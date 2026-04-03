# Test & Documentation Improvements Summary

## What Was Done

### 1. New Test Files Created

| File | Tests | Purpose |
|------|-------|---------|
| `tests/serde_json_comparison.rs` | 19 | Direct behavioral comparison with serde_json |
| `tests/error_position_tests.rs` | 6 | Error position and message quality validation |
| `docs/COMPATIBILITY_GUIDE.md` | N/A | Comprehensive compatibility documentation |

### 2. Test Files Enhanced

| File | Changes |
|------|---------|
| `tests/behavioral_parity.rs` | Removed `raw_value` requirement - now runs with just `serde` feature (5 tests vs 0 before) |
| `tests/json_test_suite.rs` | Added value correctness validation that compares parsed values against serde_json |

### 3. Documentation Created/Updated

**Created**:
- `docs/COMPATIBILITY_GUIDE.md` - Comprehensive guide covering:
  - ✅ What works (tested & verified)
  - ⚠️ Known differences (with examples)
  - ❌ What's not supported
  - Migration guide from serde_json
  - Test coverage summary

**Updated**:
- `README.md` - Added reference to compatibility guide with quick summary
- `docs/SERDE_FEATURE.md` - Added reference to compatibility guide

## Test Coverage Summary

**Total: 94 tests, all passing**

### New Tests (25 tests)
- 19 serde_json comparison tests (direct upstream comparison)
- 6 error position tests (line/column accuracy)

### Enhanced Tests
- 5 behavioral parity tests (now run by default with serde feature)
- 1 JSONTestSuite value correctness test (validates parsed values)

## What's Now Documented

### ✅ What Works
- Core JSON operations (parse/serialize)
- Typed serde (structs, enums)
- Value tree operations (indexing, pointer, take)
- Map operations (all 15+ methods)
- Error handling (line/column, categories)
- Unicode (escapes, surrogate pairs, UTF-8)
- Number handling (i64, u64, f64)
- Order preservation

### ⚠️ Known Differences
1. **Float precision**: Minor differences in edge cases (e.g., 87.3 vs 87.29999999999999716)
2. **Key ordering**: Always preserves insertion order (serde_json sorts by default)
3. **Enum deserialization**: Basic and adjacent enums work, complex patterns untested
4. **Error messages**: Similar wording but not identical
5. **Number representation**: Exposed enum vs opaque (same API behavior)
6. **Performance**: `from_str::<JsonValue>` slower with serde feature (use `parse_json` instead)

### ❌ Not Supported
- `StreamDeserializer` (streaming API)
- Low-level `Deserializer`/`Serializer`
- `value_raw()` function
- `Result<T>` type alias
- `ser`/`de` module internals

### ⚠️ Untested (May Work)
- `#[serde(untagged)]`
- `#[serde(flatten)]`
- `#[serde(skip_serializing_if)]`
- `#[serde(serialize_with)]`
- `#[serde(deserialize_with)]`
- Custom serializers/deserializers
- Ecosystem integrations (`serde_path_to_error`, `serde_with`, etc.)

## How to Verify

```bash
# Run all tests with serde feature
cargo test --features serde

# Run serde_json comparison tests specifically
cargo test --features serde --test serde_json_comparison

# Run error position tests
cargo test --features serde --test error_position_tests

# Run JSONTestSuite with value validation
cargo test --features serde --test json_test_suite test_valid_json_value_correctness

# All features
cargo test --all-features
```

## Files Changed

### Created (3 files)
- `tests/serde_json_comparison.rs` (689 lines)
- `tests/error_position_tests.rs` (318 lines)
- `docs/COMPATIBILITY_GUIDE.md` (394 lines)

### Modified (5 files)
- `Cargo.toml` - Added serde_json_upstream to dev-dependencies
- `tests/behavioral_parity.rs` - Removed raw_value requirement
- `tests/json_test_suite.rs` - Added value correctness test
- `README.md` - Added compatibility guide reference
- `docs/SERDE_FEATURE.md` - Added compatibility guide reference

## Impact

**Before**:
- 69 tests (limited serde_json comparison)
- Scattered documentation
- Unclear what works and what doesn't

**After**:
- 94 tests (25 new, comprehensive coverage)
- Centralized compatibility guide
- Clear documentation of limitations
- Direct behavioral comparison with serde_json
- Error position validation
- Value correctness validation across 95 JSONTestSuite files
