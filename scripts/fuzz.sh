#!/bin/bash
# Fuzzing script for lifegraph-json
#
# Uses libFuzzer via cargo-fuzz to find edge cases and bugs.
#
# Usage:
#   ./scripts/fuzz.sh              # Run all fuzzers briefly
#   ./scripts/fuzz.sh parse 3600   # Run parse fuzzer for 1 hour
#   ./scripts/fuzz.sh --list       # List available fuzz targets

set -e

FUZZ_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/fuzz"
CORPUS_DIR="$FUZZ_DIR/corpus"
ARTIFACTS_DIR="$FUZZ_DIR/artifacts"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== lifegraph-json Fuzzing ==="
echo ""

# Check if cargo-fuzz is installed
if ! command -v cargo-fuzz &> /dev/null; then
    echo -e "${YELLOW}cargo-fuzz not found. Installing...${NC}"
    cargo install cargo-fuzz
fi

# Create directories
mkdir -p "$CORPUS_DIR" "$ARTIFACTS_DIR"

# List available fuzz targets
list_targets() {
    echo "Available fuzz targets:"
    echo "  parse     - JSON parsing (raw bytes)"
    echo "  roundtrip - Parse + serialize roundtrip"
    echo "  tape      - Tape parsing"
}

# Handle --list flag
if [[ "$1" == "--list" ]]; then
    list_targets
    exit 0
fi

# Get target and timeout from args
TARGET="${1:-all}"
TIMEOUT="${2:-60}"  # Default 60 seconds per target

# Run a single fuzz target
run_fuzzer() {
    local target=$1
    local timeout=$2
    local corpus="$CORPUS_DIR/$target"
    local artifacts="$ARTIFACTS_DIR/$target"
    
    mkdir -p "$corpus" "$artifacts"
    
    echo ""
    echo -e "${GREEN}Running fuzzer: $target${NC} (timeout: ${timeout}s)"
    echo "Corpus: $corpus"
    echo "Artifacts: $artifacts"
    echo ""
    
    cd "$FUZZ_DIR"
    
    # Run with timeout
    # -max_total_time: stop after this many seconds
    # -artifact_prefix: where to save crash/timeout artifacts
    timeout "${timeout}s" cargo fuzz run "$target" \
        --artifact-prefix="$artifacts/" \
        --corpus="$corpus" \
        -max_total_time="$timeout" \
        || true
    
    cd - > /dev/null
    
    echo ""
    echo -e "${GREEN}Fuzzer $target completed${NC}"
}

# Check for crashes in artifacts
check_artifacts() {
    echo ""
    echo "=== Checking for artifacts ==="
    
    local found=0
    for dir in "$ARTIFACTS_DIR"/*; do
        if [[ -d "$dir" ]]; then
            local count=$(find "$dir" -type f | wc -l)
            if [[ $count -gt 0 ]]; then
                echo -e "${RED}Found $count artifacts in $dir${NC}"
                echo "  Review these for potential bugs"
                found=1
            fi
        fi
    done
    
    if [[ $found -eq 0 ]]; then
        echo -e "${GREEN}No crash artifacts found${NC}"
    fi
}

# Main execution
if [[ "$TARGET" == "all" ]]; then
    echo "Running all fuzz targets..."
    list_targets
    
    run_fuzzer "parse" "$TIMEOUT"
    run_fuzzer "roundtrip" "$TIMEOUT"
    run_fuzzer "tape" "$TIMEOUT"
    
    check_artifacts
else
    run_fuzzer "$TARGET" "$TIMEOUT"
    check_artifacts
fi

echo ""
echo "=== Fuzzing Complete ==="
echo ""
echo "To continue fuzzing a specific target:"
echo "  cd fuzz && cargo fuzz run <target>"
echo ""
echo "To minimize corpus:"
echo "  cd fuzz && cargo fuzz cmin <target>"
echo ""
echo "To analyze a crash:"
echo "  cd fuzz && cargo fuzz run <target> <artifact_file>"
