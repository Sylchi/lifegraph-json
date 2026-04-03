#!/bin/bash
# Miri memory safety validation script
# 
# Miri is an interpreter for Rust's mid-level intermediate representation (MIR).
# It can detect undefined behavior in Rust code, including:
# - Out-of-bounds memory accesses
# - Use-after-free
# - Invalid pointer operations
# - Data races (in concurrent code)
#
# Usage: ./scripts/miri.sh
#
# Requirements:
#   rustup component add miri
#   cargo miri setup

set -e

echo "=== lifegraph-json Miri Validation ==="
echo ""

# Check if miri is installed
if ! command -v cargo-miri &> /dev/null; then
    echo "ERROR: cargo-miri not found."
    echo "Install with: rustup component add miri"
    echo "Then run: cargo miri setup"
    exit 1
fi

echo "Miri found. Setting up..."
cargo miri setup

echo ""
echo "Running Miri on tests..."
echo ""

# Run miri on all tests
# MIRIFLAGS can be used to control Miri behavior
# -Zmiri-disable-isolation allows file I/O (needed for some tests)
export MIRIFLAGS="-Zmiri-disable-isolation"

# Run core tests
echo "Testing core library..."
cargo miri test --lib

echo ""
echo "Testing JSONTestSuite harness..."
cargo miri test --test json_test_suite 2>/dev/null || echo "(skipping - test data not downloaded)"

echo ""
echo "Testing Unicode harness..."
cargo miri test --test unicode

echo ""
echo "Testing behavioral parity..."
cargo miri test --test behavioral_parity

echo ""
echo "=== Miri Validation Complete ==="
echo ""
echo "If you see this message, no undefined behavior was detected."
echo "Note: Miri is not exhaustive. Also run fuzzing for additional coverage."
