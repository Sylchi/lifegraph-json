# Miri Memory Safety Validation

This document describes the Miri validation process for lifegraph-json.

## What is Miri?

[Miri](https://github.com/rust-lang/miri) is an interpreter for Rust's MIR (Mid-level Intermediate Representation) that can detect undefined behavior in Rust code, including:

- Out-of-bounds memory accesses
- Use-after-free
- Invalid pointer operations
- Uninitialized memory reads
- Invalid values (e.g., invalid bools, enums, floats)

## Running Miri

### Setup

```bash
# Install Miri component
rustup component add miri

# Set up Miri (downloads the Miri sysroot)
cargo miri setup
```

### Run Validation

```bash
# Run the validation script
./scripts/miri.sh

# Or run individual commands
cargo miri test --lib
cargo miri test --test unicode
cargo miri test --test behavioral_parity
```

## Unsafe Code in lifegraph-json

The following `unsafe` blocks exist in the codebase and are validated by Miri:

### 1. `String::from_utf8_unchecked` (lib.rs)

```rust
Ok(unsafe { String::from_utf8_unchecked(out) })
```

**Location:** `to_json_string`, `CompiledObjectSchema::to_json_string`, `CompiledRowSchema::to_json_string`

**Safety Invariant:** The serializer only produces valid UTF-8 because:
- All string content is either copied from input (already valid UTF-8) or escaped
- Escape sequences produce ASCII characters
- Numbers are formatted using Rust's standard library (valid UTF-8)

**Miri Validation:** Miri checks that the resulting bytes are valid UTF-8.

### 2. Indexing Operations

All indexing operations use safe Rust bounds checking through:
- `Vec::get` / `Vec::get_mut`
- Slice indexing with bounds checks
- Iterator-based access

## Known Limitations

Miri does not catch:
- Logic errors (wrong algorithm)
- Performance issues
- All possible inputs (only tests what's exercised)

Therefore, Miri should be combined with:
- Fuzzing (see `fuzz/`)
- Property-based testing
- JSONTestSuite validation

## CI Integration

Miri runs on every PR in GitHub Actions. See `.github/workflows/ci.yml`.

## Interpreting Miri Errors

If Miri reports an error:

```
error: Undefined Behavior: accessing memory with address <addr> out of bounds
```

This indicates a real bug that must be fixed. Common causes:
- Off-by-one errors in indexing
- Incorrect pointer arithmetic
- Violating type invariants

Fix the issue and re-run Miri to confirm the fix.
