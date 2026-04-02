//! Fuzz target: JSON parsing
//!
//! Tests that parsing arbitrary input doesn't panic or cause UB.
//! We don't care if it fails - we care that it fails safely.

#![no_main]

use libfuzzer_sys::fuzz_target;
use lifegraph_json::{from_slice, JsonValue};

fuzz_target!(|data: &[u8]| {
    // Convert to string (may fail for invalid UTF-8, which is fine)
    if let Ok(input) = std::str::from_utf8(data) {
        // Try to parse - we don't care if it succeeds, just that it doesn't panic
        let _ = from_slice::<JsonValue>(input.as_bytes());
    }
    
    // Also test from_slice directly on raw bytes
    let _ = from_slice::<JsonValue>(data);
});
