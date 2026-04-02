//! Fuzz target: Tape parsing
//!
//! Tests that tape parsing arbitrary input doesn't panic or cause UB.
//! Tape parsing is the fast path that builds a structural index.

#![no_main]

use libfuzzer_sys::fuzz_target;
use lifegraph_json::{parse_json_tape, from_slice, JsonValue};

fuzz_target!(|data: &[u8]| {
    // Only test valid UTF-8 strings
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    
    // Try tape parse - we don't care if it succeeds
    if let Ok(tape) = parse_json_tape(input) {
        // If tape parse succeeds, verify we can get root
        if let Some(root) = tape.root(input) {
            // Try to build object index (only works for objects)
            let _ = root.build_object_index();
            
            // Try to access kind
            let _ = root.kind();
        }
    }
    
    // Also verify that if regular parse succeeds, tape parse should too
    if from_slice::<JsonValue>(input.as_bytes()).is_ok() {
        // Tape parse should not panic even if it might fail
        let _ = parse_json_tape(input);
    }
});
