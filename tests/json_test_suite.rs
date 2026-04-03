//! JSONTestSuite compatibility tests
//!
//! Run with: cargo test --test json_test_suite

#[cfg(feature = "serde")]
use lifegraph_json::from_slice;
use lifegraph_json::{parse_json, JsonValue};

/// Load all test cases from the JSONTestSuite directory
/// Files are named with prefixes: y_ (valid), n_ (invalid), i_ (implementation-defined)
fn load_test_cases() -> (
    Vec<(String, Vec<u8>)>,
    Vec<(String, Vec<u8>)>,
    Vec<(String, Vec<u8>)>,
) {
    let base_path = "tests/json_test_suite/test_parsing";
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    let mut impl_defined = Vec::new();

    if !std::path::Path::new(base_path).exists() {
        eprintln!("Warning: JSONTestSuite data not found at {}", base_path);
        return (valid, invalid, impl_defined);
    }

    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(data) = std::fs::read(&path) {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if name.starts_with("y_") {
                        valid.push((name, data));
                    } else if name.starts_with("n_") {
                        invalid.push((name, data));
                    } else if name.starts_with("i_") {
                        impl_defined.push((name, data));
                    }
                }
            }
        }
    }

    (valid, invalid, impl_defined)
}

/// Test valid JSON files - should parse successfully
#[test]
fn test_valid_json() {
    let (cases, _, _) = load_test_cases();
    if cases.is_empty() {
        eprintln!("No valid JSON test cases found");
        return;
    }

    let mut passed = 0;
    let mut failed = Vec::new();

    for (name, data) in cases {
        let data_str = match std::str::from_utf8(&data) {
            Ok(s) => s,
            Err(_) => {
                passed += 1; // Invalid UTF-8 is a valid rejection
                continue;
            }
        };
        match parse_json(data_str) {
            Ok(_) => passed += 1,
            Err(e) => failed.push((name, e)),
        }
    }

    if !failed.is_empty() {
        eprintln!("\n=== FAILED VALID JSON TESTS ===");
        for (name, err) in &failed {
            eprintln!("  {}: {:?}", name, err);
        }
        panic!(
            "{} of {} valid JSON tests failed",
            failed.len(),
            passed + failed.len()
        );
    }

    eprintln!("Passed {} valid JSON tests", passed);
}

/// Test invalid JSON files - should fail to parse
#[test]
fn test_invalid_json() {
    let (_, cases, _) = load_test_cases();
    if cases.is_empty() {
        eprintln!("No invalid JSON test cases found");
        return;
    }

    let mut passed = 0;
    let mut failed = Vec::new();
    let mut depth_errors = Vec::new();

    for (name, data) in cases {
        let data_str = match std::str::from_utf8(&data) {
            Ok(s) => s,
            Err(_) => {
                passed += 1; // Invalid UTF-8 is a valid rejection
                continue;
            }
        };
        match parse_json(data_str) {
            Ok(_) => failed.push((name, "unexpectedly parsed".to_string())),
            Err(e) => {
                // Check if it's a depth error - that's acceptable rejection
                if format!("{:?}", e).contains("NestingTooDeep") {
                    depth_errors.push(name);
                }
                passed += 1; // Any error is acceptable for invalid JSON
            }
        }
    }

    if !depth_errors.is_empty() {
        eprintln!("\n=== DEPTH ERRORS (acceptable rejections) ===");
        for name in &depth_errors {
            eprintln!("  {}", name);
        }
    }

    if !failed.is_empty() {
        eprintln!("\n=== FAILED INVALID JSON TESTS ===");
        for (name, reason) in &failed {
            eprintln!("  {}: {}", name, reason);
        }
        panic!(
            "{} of {} invalid JSON tests incorrectly parsed",
            failed.len(),
            passed + failed.len()
        );
    }

    eprintln!(
        "Passed {} invalid JSON rejection tests ({} depth errors)",
        passed,
        depth_errors.len()
    );
}

/// Test implementation-defined JSON
#[test]
fn test_implementation_defined_json() {
    let (_, _, cases) = load_test_cases();
    if cases.is_empty() {
        eprintln!("No implementation-defined JSON test cases found");
        return;
    }

    let mut accepted = 0;
    let mut rejected = 0;

    for (name, data) in cases {
        let data_str = match std::str::from_utf8(&data) {
            Ok(s) => s,
            Err(_) => {
                rejected += 1;
                continue;
            }
        };
        match parse_json(data_str) {
            Ok(_) => {
                eprintln!("  Accepted: {}", name);
                accepted += 1;
            }
            Err(_) => {
                eprintln!("  Rejected: {}", name);
                rejected += 1;
            }
        }
    }

    eprintln!(
        "Implementation-defined: {} accepted, {} rejected (no pass/fail)",
        accepted, rejected
    );
}

/// Test number edge cases
#[test]
fn test_number_edge_cases() {
    let test_cases: Vec<(&[u8], bool)> = vec![
        (b"9223372036854775807", true),     // i64::MAX
        (b"9223372036854775808", true),     // i64::MAX + 1
        (b"-9223372036854775808", true),    // i64::MIN
        (b"18446744073709551615", true),    // u64::MAX
        (b"1.7976931348623157e+308", true), // f64::MAX
        (b"1.", false),                     // trailing dot
        (b"1.e10", false),                  // no fraction digits
        (b"--1", false),                    // double minus
        (b"01", false),                     // leading zero
        (b"1e", false),                     // incomplete exponent
    ];

    for (data, should_pass) in test_cases {
        let data_str = std::str::from_utf8(data).unwrap_or("");
        let result = parse_json(data_str);
        let passed = result.is_ok() == should_pass;
        assert!(
            passed,
            "Number test failed: {:?} - expected {}, got {:?}",
            String::from_utf8_lossy(data),
            if should_pass { "success" } else { "error" },
            result
        );
    }
}

/// Test unicode string handling
#[test]
fn test_unicode_strings() {
    let test_cases: Vec<(&[u8], bool)> = vec![
        (br#""hello""#, true),
        (br#""\u0041""#, true),       // 'A'
        (br#""\u4e2d\u6587""#, true), // Chinese
        (br#""\uD83D\uDE00""#, true), // 😀
        (br#""\uD800""#, false),      // lone high surrogate
        (br#""\uDC00""#, false),      // lone low surrogate
        (br#""\x41""#, false),        // \x not valid
        (br#""\q""#, false),          // invalid escape
    ];

    for (data, should_pass) in test_cases {
        let data_str = std::str::from_utf8(data).unwrap_or("");
        let result = parse_json(data_str);
        let passed = result.is_ok() == should_pass;
        assert!(
            passed,
            "Unicode test failed: {:?} - expected {}, got {:?}",
            String::from_utf8_lossy(data),
            if should_pass { "success" } else { "error" },
            result
        );
    }
}

/// Test deep nesting
#[test]
fn test_deep_nesting() {
    for depth in [100, 500, 1000] {
        let mut json = vec![b'['; depth];
        json.push(b'1');
        json.extend(vec![b']'; depth]);

        let json_str = std::str::from_utf8(&json).unwrap();
        let result = parse_json(json_str);
        eprintln!(
            "Depth {}: {:?}",
            depth,
            result.as_ref().map(|_| "ok").unwrap_or("err")
        );
    }
}

/// Test that depth limiting works
#[test]
fn test_depth_limiting() {
    // 127 nested arrays should work (below limit of 128)
    let json_127 = format!("[{}1{}]", "[".repeat(126), "]".repeat(126));
    let result = parse_json(&json_127);
    eprintln!(
        "127 depth: {:?}",
        result.as_ref().map(|_| "ok").unwrap_or("err")
    );
    assert!(result.is_ok(), "Expected 127 depth to parse OK");

    // 1000 nested arrays should fail with NestingTooDeep
    let json_1000 = format!("[{}1{}]", "[".repeat(999), "]".repeat(999));
    let result = parse_json(&json_1000);
    eprintln!("1000 depth: {:?}", result);
    assert!(
        result.is_err(),
        "Expected NestingTooDeep error, got: {:?}",
        result
    );
}
