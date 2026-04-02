# Fuzzing Guide

This document describes how to use fuzzing to find bugs in lifegraph-json.

## What is Fuzzing?

Fuzzing is an automated testing technique that provides random, invalid, or unexpected input to a program to find:
- Crashes (panics)
- Memory safety violations
- Infinite loops / hangs
- Logic errors

We use [libFuzzer](https://llvm.org/docs/LibFuzzer.html) via [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz).

## Quick Start

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Run all fuzzers for 60 seconds each
./scripts/fuzz.sh

# Run a specific fuzzer for 1 hour
./scripts/fuzz.sh parse 3600

# Run indefinitely (until Ctrl+C)
cd fuzz && cargo fuzz run parse
```

## Fuzz Targets

### `parse`
Tests JSON parsing on arbitrary byte sequences. Catches:
- Panics on malformed input
- Incorrect error handling
- Edge cases in number/string parsing

### `roundtrip`
Tests that `parse(serialize(parse(input))) == parse(input)`. Catches:
- Serialization bugs
- Non-deterministic parsing
- Data loss during round-trip

### `tape`
Tests tape parsing specifically. Catches:
- Tape construction bugs
- Index building errors
- Token navigation issues

## Interpreting Results

### Success
If the fuzzer runs without finding issues:
```
Running: parse (timeout: 60s)
...
Fuzzer parse completed
No crash artifacts found
```

This is good! But doesn't prove correctness - just that no bugs were found in the tested inputs.

### Crash Found
If the fuzzer finds a crash:
```
==12345== ERROR: libFuzzer: deadly signal
    #0 0x... in panic
    #1 0x... in lifegraph_json::parse
```

The fuzzer saves the crashing input to `fuzz/artifacts/parse/crash-<hash>`.

To reproduce:
```bash
cd fuzz
cargo fuzz run parse artifacts/parse/crash-<hash>
```

To debug:
```bash
# Run with RUST_BACKTRACE
RUST_BACKTRACE=1 cargo fuzz run parse artifacts/parse/crash-<hash>

# Or use gdb/lldb
gdb --args target/x86_64-unknown-linux-gnu/release/parse artifacts/parse/crash-<hash>
```

### Timeout Found
Timeouts indicate potential performance issues or infinite loops:
```
==12345== ERROR: libFuzzer: timeout
```

Timeout inputs are saved to `fuzz/artifacts/parse/timeout-<hash>`.

## Corpus Management

### Minimize Corpus
Reduce the corpus to the smallest set of inputs that provide the same coverage:
```bash
cd fuzz && cargo fuzz cmin parse
```

### Merge New Inputs
After adding seed inputs:
```bash
cd fuzz && cargo fuzz merge
```

### Add Seed Inputs
Good seeds help the fuzzer explore more efficiently:
```bash
# Add interesting test cases to corpus
cp tests/fixtures/*.json fuzz/corpus/parse/
```

## Continuous Fuzzing

For production projects, consider:
- [OSS-Fuzz](https://github.com/google/oss-fuzz) - Google's continuous fuzzing service
- Running fuzzers in CI (with time limits)
- Periodic fuzzing runs (e.g., nightly)

## Combining with Other Tools

Fuzzing complements but doesn't replace:
- **Miri** - Catches UB that fuzzing might miss
- **JSONTestSuite** - Systematic correctness testing
- **Unit tests** - Targeted edge case testing
- **Property tests** - Formal property verification

Recommended workflow:
1. Write unit tests for known edge cases
2. Run Miri to catch UB
3. Run JSONTestSuite for correctness
4. Fuzz for unknown edge cases

## Tips

1. **Run longer** - More time = more coverage. Aim for hours, not seconds.
2. **Use multiple targets** - Different targets find different bugs.
3. **Check artifacts** - Even if fuzzer doesn't crash, artifacts may reveal issues.
4. **Seed with real data** - Real JSON helps fuzzer start from valid states.
5. **Monitor coverage** - Use `cargo fuzz coverage` to see what's being tested.

## Reporting Bugs

If fuzzing finds a bug:
1. Save the artifact (crashing input)
2. Create a minimal reproduction
3. Add as a regression test
4. Fix the bug
5. Verify fix with fuzzer

## Current Status

| Target | Last Run | Duration | Issues Found |
|--------|----------|----------|--------------|
| parse  | -        | -        | -            |
| roundtrip | -     | -        | -            |
| tape   | -        | -        | -            |

*Update this table after each fuzzing session.*
