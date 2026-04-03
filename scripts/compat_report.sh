#!/bin/bash
# Generates serde_json API compatibility report for lifegraph-json
# Usage: ./scripts/compat_report.sh
set -euo pipefail

cd "$(dirname "$0")/.."

echo "============================================="
echo "  lifegraph-json → serde_json API Parity"
echo "============================================="
echo ""

# Check if a public item exists by name
has_pub() {
    local item="$1"
    grep -rq "pub fn $item\b\|pub struct $item\b\|pub enum $item\b\|pub type $item\b\|pub use.*$item\b\|macro_rules! $item" \
        src/lib.rs src/serde_api.rs src/serde_error.rs src/serde_deserialize.rs \
        src/serde_serialize.rs src/value.rs src/number.rs src/map.rs src/json_macro.rs 2>/dev/null
}

ROWFMT="%-30s %s  %s\n"

echo "--- Top-Level Functions ---"
for fn in from_str from_slice from_reader from_value to_string to_vec to_writer \
          to_string_pretty to_vec_pretty to_writer_pretty to_value; do
    if has_pub "$fn"; then
        printf "$ROWFMT" "$fn" "✅" ""
    else
        printf "$ROWFMT" "$fn" "❌" "Missing"
    fi
done
echo ""

echo "--- Types ---"
for ty in Value Map Number Error Result; do
    if has_pub "$ty"; then
        printf "$ROWFMT" "$ty" "✅" ""
    else
        printf "$ROWFMT" "$ty" "❌" "Missing"
    fi
done
echo ""

echo "--- Macros ---"
for mac in json; do
    if has_pub "$mac"; then
        printf "$ROWFMT" "${mac}!" "✅" ""
    else
        printf "$ROWFMT" "${mac}!" "❌" "Missing"
    fi
done
echo ""

echo "--- Raw Value (raw_value feature) ---"
for item in RawValue to_raw_value; do
    if has_pub "$item"; then
        printf "$ROWFMT" "$item" "✅" ""
    else
        printf "$ROWFMT" "$item" "❌" "Missing"
    fi
done
echo ""

echo "--- Not Yet Implemented ---"
printf "$ROWFMT" "StreamDeserializer" "❌" "Iterator over consecutive JSON values"
printf "$ROWFMT" "Deserializer" "❌" "Low-level (internal only)"
printf "$ROWFMT" "Serializer" "❌" "Low-level (internal only)"
printf "$ROWFMT" "ser module" "❌" "Serialization utilities"
printf "$ROWFMT" "de module" "❌" "Deserialization utilities"
echo ""

echo "--- Feature Gate Analysis ---"
DEFAULT=$(grep '^default' Cargo.toml | head -1 | sed 's/default = //' | tr -d '[]"' | tr -d ' ')
SERDE_IN_DEFAULT=$(echo "$DEFAULT" | grep -c "serde" || true)
echo "  default features:  $DEFAULT"
echo "  serde_json default: serde (always enabled)"
if [ "$SERDE_IN_DEFAULT" -gt 0 ]; then
    echo "  ✅ serde is in default features — drop-in compatible"
else
    echo "  ⚠️  serde is NOT in default — breaks drop-in compatibility"
    echo "     Recommendation: add serde to default features"
fi
echo ""

echo "--- Behavioral Parity Tests ---"
if cargo test --all-features --test behavioral_parity 2>/dev/null | grep -q "test result: ok"; then
    echo "  behavioral_parity:  ✅ PASS"
else
    echo "  behavioral_parity:  ❌ FAIL or not enabled"
fi
echo ""
echo "============================================="
