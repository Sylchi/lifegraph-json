# lifegraph-json Quick Reference

## Getting Started

```bash
# Clone and setup
git clone https://github.com/Sylchi/lifegraph-json
cd lifegraph-json
./scripts/setup.sh

# Build and test
cargo build
cargo test
```

---

## Daily Development

```bash
# Before committing
cargo fmt
cargo check

# Commit (runs pre-commit hooks automatically)
git commit -m "feat: description"

# Before pushing
cargo test
cargo clippy

# Push (runs pre-push hooks automatically)
git push
```

---

## Feature Branch Workflow

```bash
# Create feature branch
git checkout main
git pull
git checkout -b feature/my-feature

# Develop and commit
git commit -m "feat: add feature"

# Push and create PR
git push -u origin feature/my-feature
# Then create PR on GitHub

# After review, merge via GitHub UI
```

---

## Testing

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# JSONTestSuite
cargo test --test json_test_suite

# Unicode tests
cargo test --test unicode

# Benchmarks
cargo bench

# Miri (memory safety)
./scripts/miri.sh

# Fuzzing (60 seconds each)
./scripts/fuzz.sh
```

---

## Git Hooks

| Hook | Runs | Time |
|------|------|------|
| pre-commit | `cargo fmt --check`, `cargo check` | ~5s |
| pre-push | `cargo build --release`, `cargo test` | ~60s |
| post-merge | `cargo check` | ~5s |

Install: `git config core.hooksPath .githooks`

---

## Release

```bash
# Create release (updates version, changelog, tags, pushes)
./scripts/release.sh 1.0.150

# CI automatically:
# - Runs all tests
# - Publishes to crates.io
# - Creates GitHub release
```

---

## Common Commands

| Command | Purpose |
|---------|---------|
| `cargo build` | Build debug |
| `cargo build --release` | Build release |
| `cargo test` | Run all tests |
| `cargo test --lib` | Library tests only |
| `cargo clippy` | Linting |
| `cargo fmt` | Formatting |
| `cargo fmt --check` | Check formatting |
| `cargo doc --open` | Build and open docs |
| `cargo bench` | Run benchmarks |

---

## Project Structure

```
lifegraph-json/
├── src/
│   ├── lib.rs          # Main library
│   ├── parse.rs        # Parser (if used)
│   ├── error.rs        # Error types
│   └── ...
├── tests/
│   ├── json_test_suite.rs
│   ├── unicode.rs
│   └── ...
├── benches/
├── fuzz/
├── docs/
│   ├── BRANCH_PROTECTION.md
│   ├── FUZZING.md
│   └── MIRI.md
├── scripts/
│   ├── setup.sh
│   ├── miri.sh
│   ├── fuzz.sh
│   └── release.sh
└── .githooks/
```

---

## Troubleshooting

### Hook not running?
```bash
git config core.hooksPath .githooks
```

### Tests failing after merge?
```bash
cargo clean
cargo test
```

### Format issues?
```bash
cargo fmt
```

### Clippy warnings?
```bash
cargo clippy --fix
```

---

## Documentation

- [Branch Protection & Workflow](docs/BRANCH_PROTECTION.md)
- [Fuzzing Guide](docs/FUZZING.md)
- [Miri Guide](docs/MIRI.md)
- [Setup Guide](SETUP.md)
- [Status](STATUS.md)

---

## Getting Help

- **Issues**: https://github.com/Sylchi/lifegraph-json/issues
- **PRs**: https://github.com/Sylchi/lifegraph-json/pulls
- **Docs**: https://docs.rs/lifegraph-json
