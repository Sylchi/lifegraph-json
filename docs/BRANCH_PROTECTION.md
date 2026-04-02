# Branch Protection & Workflow

## Branch Structure

```
main              - Protected, production-ready code
├── feature/*     - New features
├── fix/*         - Bug fixes  
├── refactor/*    - Code improvements
└── release/*     - Release preparation
```

---

## GitHub Branch Protection Rules

Configure these in **GitHub → Settings → Branches → Add rule**:

### For `main` branch:

**Branch name pattern:** `main`

**Protect matching branches:**
- [x] Require a pull request before merging
  - [x] Require approvals: **1**
  - [x] Dismiss stale pull request approvals when new commits are pushed
- [x] Require status checks to pass before merging
  - [x] CI (ubuntu-latest)
  - [x] Clippy
  - [x] Rustfmt
- [x] Require branches to be up to date before merging
- [x] Include administrators (optional)
- [x] Force pushes: **Block**
- [x] Deletions: **Block**

**Status checks required:**
- `CI (ubuntu-latest)` - from `.github/workflows/ci.yml`
- `Clippy` - from `.github/workflows/ci.yml`
- `Rustfmt` - from `.github/workflows/ci.yml`

---

## Git Hooks (Local)

Install hooks:
```bash
git config core.hooksPath .githooks
```

### Pre-commit (< 5 seconds)
- Format check (`cargo fmt --check`)
- Compile check (`cargo check`)

### Pre-push (< 60 seconds)
- Release build (`cargo build --release`)
- Library tests (`cargo test --lib`)
- Integration tests (`cargo test --tests`)

---

## Feature Branch Workflow

### 1. Create Feature Branch

```bash
# Start from latest main
git checkout main
git pull

# Create feature branch
git checkout -b feature/my-new-feature
```

### 2. Develop

```bash
# Make changes
# ...

# Fast pre-commit checks run automatically
git commit -m "feat: add new feature"

# Continue developing
git commit -m "fix: address review feedback"
```

### 3. Before Pushing

```bash
# Run full test suite locally
cargo test

# Run clippy
cargo clippy --all-targets

# Run benchmarks (optional)
cargo bench
```

### 4. Push and Create PR

```bash
git push -u origin feature/my-new-feature
```

Then on GitHub:
1. Go to **Pull Requests** → **New Pull Request**
2. Select your feature branch
3. Fill in PR template
4. Request review

### 5. Address Review Feedback

```bash
# Make requested changes
git commit -m "review: address feedback"

# Push (PR updates automatically)
git push
```

### 6. Merge

Once CI passes and approval is granted:
- **Squash and merge** (preferred for feature branches)
- **Rebase and merge** (for small fixes)

---

## Hotfix Workflow

For urgent production fixes:

```bash
# Branch from main
git checkout main
git pull
git checkout -b fix/critical-bug

# Fix and test
# ...
git commit -m "fix: critical bug description"

# Push and create PR with [HOTFIX] prefix
git push -u origin fix/critical-bug
```

Label PR as **hotfix** for expedited review.

---

## Release Workflow

### 1. Create Release Branch

```bash
git checkout main
git pull
git checkout -b release/v1.0.150
```

### 2. Update Version

```bash
./scripts/release.sh 1.0.150
```

This:
- Updates `Cargo.toml` version
- Updates `CHANGELOG.md`
- Creates git tag
- Pushes to trigger CI

### 3. Verify CI

Watch GitHub Actions:
- CI must pass on all platforms
- Release workflow publishes to crates.io
- GitHub release is created automatically

---

## Commit Message Conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new feature
fix: fix bug in parser
docs: update README
refactor: improve error handling
test: add test for edge case
chore: update dependencies
```

**Breaking changes:**
```
feat!: change API signature

BREAKING CHANGE: function signature changed
```

---

## CI/CD Pipeline

```
Push to feature/*
  └─→ CI runs:
      ├─→ Test (Ubuntu, Windows, macOS)
      ├─→ Clippy
      ├─→ Rustfmt
      └─→ MSRV check

PR Created
  └─→ All status checks must pass
      └─→ Requires 1 approval
          └─→ Merge to main

Push to main (tag v*)
  └─→ Release workflow:
      ├─→ Verify build
      ├─→ Publish to crates.io
      └─→ Create GitHub release
```

---

## Troubleshooting

### Hook not running?
```bash
# Verify hooks are installed
git config core.hooksPath
# Should output: .githooks

# Reinstall if needed
git config core.hooksPath .githooks
```

### CI failing but tests pass locally?
```bash
# Run same environment as CI
cargo test --all-features
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Need to bypass hooks (emergency only)?
```bash
git commit --no-verify -m "emergency fix"
```

**Use sparingly** - hooks exist for quality reasons.

---

## Quick Reference

| Command | Purpose |
|---------|---------|
| `git checkout -b feature/x` | Create feature branch |
| `git commit` | Runs pre-commit hooks |
| `git push` | Runs pre-push hooks |
| `cargo test` | Full test suite |
| `cargo clippy` | Linting |
| `cargo fmt` | Formatting |
| `./scripts/release.sh` | Create release |
