#!/bin/bash
# Setup script for lifegraph-json
#
# Installs git hooks and sets up development environment.
# Run this after cloning the repository.

set -e

echo "=== lifegraph-json Setup ==="
echo ""

# Get the directory where this script lives
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# Install git hooks
echo "Installing git hooks..."
git config core.hooksPath "$ROOT_DIR/.githooks"
echo "✓ Git hooks installed (using .githooks directory)"
echo ""
echo "  Pre-commit: Format + compile check (~5s)"
echo "  Pre-push: Release build + tests (~60s)"
echo "  Post-merge: Quick verification"

# Check Rust version
echo ""
echo "Checking Rust version..."
RUST_VERSION=$(rustc --version 2>/dev/null || echo "not installed")
echo "Rust: $RUST_VERSION"

# Install recommended tools
echo ""
echo "Checking for recommended tools..."

# cargo-fuzz
if command -v cargo-fuzz &> /dev/null; then
    echo "✓ cargo-fuzz installed"
else
    echo "⚠ cargo-fuzz not found. Install with: cargo install cargo-fuzz"
fi

# cargo-miri
if rustup component list 2>/dev/null | grep -q "miri.*installed"; then
    echo "✓ Miri installed"
else
    echo "⚠ Miri not found. Install with: rustup component add miri"
fi

# cargo-audit (optional security auditing)
if command -v cargo-audit &> /dev/null; then
    echo "✓ cargo-audit installed"
else
    echo "⚠ cargo-audit not found. Install with: cargo install cargo-audit"
fi

# Download JSONTestSuite data (optional)
echo ""
echo "JSONTestSuite data..."
if [[ -d "$ROOT_DIR/tests/json_test_suite" ]]; then
    echo "✓ JSONTestSuite data found"
else
    echo "⚠ JSONTestSuite data not found."
    echo "  Download with:"
    echo "    mkdir -p tests/json_test_suite"
    echo "    curl -L https://github.com/nst/JSONTestSuite/archive/refs/heads/master.tar.gz | tar xz -C tests/json_test_suite --strip-components=1"
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "  1. Run 'cargo build' to build the project"
echo "  2. Run 'cargo test' to run tests"
echo "  3. Run 'cargo bench' to run benchmarks"
echo ""
echo "For safety validation:"
echo "  ./scripts/miri.sh    - Run Miri memory safety checks"
echo "  ./scripts/fuzz.sh    - Run fuzzing"
echo ""
echo "For branch workflow:"
echo "  See docs/BRANCH_PROTECTION.md"
echo ""
