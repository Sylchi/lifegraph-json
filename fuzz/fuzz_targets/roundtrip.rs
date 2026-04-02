//! Fuzz target: Parse + serialize roundtrip
//!
//! Tests that parse(serialize(parse(input))) == parse(input)
//! This catches bugs where serialization produces invalid JSON
//! or where parsing is non-deterministic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use lifegraph_json::{from_slice, to_string, JsonValue};

fuzz_target!(|data: &[u8]| {
    // Only test valid UTF-8 strings
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    
    // First parse
    let Ok(value) = from_slice::<JsonValue>(input.as_bytes()) else {
        return;
    };
    
    // Serialize
    let Ok(serialized) = to_string(&value) else {
        return;
    };
    
    // Parse again
    let Ok(value2) = from_slice::<JsonValue>(serialized.as_bytes()) else {
        panic!("Failed to parse serialized JSON: {}", serialized);
    };
    
    // Compare - should be equal
    assert_eq!(value, value2, "Roundtrip mismatch:\n  original: {:?}\n  serialized: {}\n  reparsed: {:?}", value, serialized, value2);
    
    // Also test that serializing again produces the same output
    let Ok(serialized2) = to_string(&value2) else {
        return;
    };
    
    assert_eq!(serialized, serialized2, "Double serialize mismatch");
});
