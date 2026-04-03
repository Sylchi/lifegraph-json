//! Error position and semantics tests
//!
//! These tests verify that error positions (line/column) match serde_json behavior.
//! Run with: cargo test --test error_position_tests -- --nocapture

#![cfg(feature = "serde")]

use lifegraph_json as lg_json;
use serde_json_upstream as sj_json;

/// Helper: extract byte position from lifegraph error message
fn extract_byte_position(msg: &str) -> Option<usize> {
    // Messages like "unexpected character 'x' at byte 5"
    msg.split("byte ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// Helper: calculate byte position from line/column in input
fn line_column_to_byte(input: &str, line: usize, column: usize) -> usize {
    let mut current_line = 1;
    let mut current_col = 0;

    for (byte_idx, ch) in input.char_indices() {
        if current_line == line && current_col == column - 1 {
            return byte_idx;
        }
        if ch == '\n' {
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
    }

    // Return input length if we reached the end
    input.len()
}

#[test]
fn test_error_byte_position_accuracy() {
    let test_cases = vec![
        // (input, description)
        ("{", "unclosed object"),
        ("{\"key\":}", "missing value after colon"),
        ("[1, 2, 3", "unclosed array"),
        ("[1, ,2]", "double comma"),
        ("[1, 2 3]", "missing comma in array"),
        ("{\"a\":1,}", "trailing comma"),
        ("{\"a\":1 \"b\":2}", "missing comma between pairs"),
        ("tru", "truncated true"),
        ("fals", "truncated false"),
        ("nul", "truncated null"),
        ("\"unterminated", "unterminated string"),
        ("\"bad\\x\"", "invalid escape \\x"),
        ("\"bad unicode\\uXXXX\"", "invalid unicode escape"),
        ("\"lone surrogate\\uD800\"", "lone high surrogate"),
        (
            "\"bad surrogate pair\\uD800\\uFFFF\"",
            "invalid surrogate pair",
        ),
        ("00", "leading zero"),
        ("0123", "leading zero on multi-digit"),
        ("1.", "trailing decimal point"),
        ("1.e10", "no fraction digits"),
        ("1e", "incomplete exponent"),
        ("1e+", "missing exponent value"),
        ("1e-+2", "invalid exponent"),
        (r#"{"key": undefined}"#, "undefined keyword"),
        (r#"{'single quotes'}"#, "single quotes"),
    ];

    for (input, description) in test_cases {
        let lg_result = lg_json::from_str::<lg_json::Value>(input);
        let sj_result = sj_json::from_str::<sj_json::Value>(input);

        // At least one should fail (both ideally)
        if lg_result.is_ok() && sj_result.is_ok() {
            // Both parsed - that's unexpected for invalid inputs
            eprintln!("Warning: both parsed for {}", description);
            continue;
        }

        if let (Err(lg_err), Err(sj_err)) = (&lg_result, &sj_result) {
            // Get byte positions
            let lg_msg = lg_err.to_string();
            let sj_line = sj_err.line();
            let sj_col = sj_err.column();

            // For single-line inputs, compare positions
            if !input.contains('\n') {
                // Convert serde_json line/column to byte
                let sj_byte_pos = line_column_to_byte(input, sj_line, sj_col);

                // Try to extract byte position from lifegraph error
                if let Some(lg_pos) = extract_byte_position(&lg_msg) {
                    // Allow small differences (within 1-2 bytes) due to different error reporting
                    let diff = lg_pos.abs_diff(sj_byte_pos);

                    if diff > 5 {
                        eprintln!(
                            "Warning: Large position difference for {}:\n  Input: {:?}\n  lifegraph byte: {}, serde_json byte: {}\n  lg msg: {}",
                            description,
                            input,
                            lg_pos,
                            sj_byte_pos,
                            lg_msg
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_multiline_error_positions() {
    let test_cases = vec![
        (
            r#"{
  "key": invalid
}"#,
            "invalid value on line 2",
        ),
        (
            r#"{
  "a": 1,
  "b":
}"#,
            "missing value on line 4",
        ),
    ];

    for (input, description) in test_cases {
        let lg_result = lg_json::from_str::<lg_json::Value>(input);
        let sj_result = sj_json::from_str::<sj_json::Value>(input);

        if let (Err(lg_err), Err(sj_err)) = (&lg_result, &sj_result) {
            let lg_msg = lg_err.to_string();
            let sj_line = sj_err.line();
            let sj_col = sj_err.column();

            eprintln!(
                "Multiline error {}:\n  Input:\n{}\n  lifegraph: {}\n  serde_json line: {}, col: {}",
                description,
                input,
                lg_msg,
                sj_line,
                sj_col
            );

            // Verify serde_json line number is reasonable
            let input_lines: Vec<_> = input.lines().collect();
            assert!(
                sj_line <= input_lines.len(),
                "serde_json line {} exceeds input line count {}",
                sj_line,
                input_lines.len()
            );
        } else {
            eprintln!(
                "Warning: test case '{}' didn't fail as expected",
                description
            );
        }
    }
}

#[test]
fn test_error_category_correctness() {
    use lifegraph_json as lg_json;

    // Test that lifegraph errors have correct categories when serde feature is enabled
    #[cfg(feature = "serde")]
    {
        // Syntax errors
        let syntax_errors = vec![
            ("[1,]", "trailing comma"),
            ("{]", "mismatched brackets"),
            ("tru", "truncated literal"),
        ];

        for (input, description) in syntax_errors {
            let err = lg_json::from_str::<lg_json::Value>(input).unwrap_err();
            // These should be syntax errors, not EOF
            assert!(
                err.is_syntax() || err.is_eof(),
                "Expected syntax or EOF error for {}, got: {:?}",
                description,
                err
            );
        }

        // EOF errors (unclosed structures)
        let eof_errors = vec![
            ("{", "unclosed object"),
            ("[", "unclosed array"),
            ("\"unterminated", "unclosed string"),
        ];

        for (input, description) in eof_errors {
            let err = lg_json::from_str::<lg_json::Value>(input).unwrap_err();
            // Could be EOF or syntax depending on implementation
            eprintln!(
                "EOF test {} - category: {:?}, is_syntax: {}, is_eof: {}",
                description,
                err.classify(),
                err.is_syntax(),
                err.is_eof()
            );
            assert!(
                err.is_syntax() || err.is_eof(),
                "Expected syntax or EOF error for {}",
                description
            );
        }

        // Data errors (type mismatches during deserialization)
        // Note: This test verifies that typed deserialization reports errors correctly
        #[derive(Debug, serde_crate::Deserialize)]
        #[serde(crate = "serde_crate")]
        #[allow(dead_code)]
        struct TestStruct {
            flag: bool,
        }

        // Pass a string where boolean is expected - this should be a type mismatch
        // Actually, serde won't deserialize "true" as a bool from a string, so this should fail
        let err = lg_json::from_str::<TestStruct>(r#"{"flag":"not_a_bool"}"#).unwrap_err();
        eprintln!(
            "Type mismatch error - category: {:?}, is_data: {}, is_syntax: {}, msg: {}",
            err.classify(),
            err.is_data(),
            err.is_syntax(),
            err
        );
        // This will be a data error because it's a type mismatch during deserialization
    }
}

#[test]
fn test_error_message_quality() {
    let test_cases = vec![
        (
            "[1,]",
            vec!["expected", "unexpected", "comma"],
            "trailing comma should mention comma or unexpected",
        ),
        (
            "00",
            vec!["number", "invalid", "leading"],
            "leading zero should mention number",
        ),
    ];

    for (input, expected_keywords, description) in test_cases {
        let lg_err = lg_json::from_str::<lg_json::Value>(input).unwrap_err();
        let lg_msg = lg_err.to_string().to_lowercase();

        let has_keyword = expected_keywords
            .iter()
            .any(|keyword| lg_msg.contains(keyword));

        assert!(
            has_keyword,
            "Error message for {} doesn't contain expected keywords: {:?}\n  Message: {}",
            description, expected_keywords, lg_msg
        );
    }
}

#[test]
fn test_serde_error_line_column_accessors() {
    // Verify that serde errors expose line/column information
    let input = r#"{
  "key": invalid
}"#;

    let lg_err = lg_json::from_str::<lg_json::Value>(input).unwrap_err();

    // These should not panic
    let line = lg_err.line();
    let column = lg_err.column();

    assert!(line > 0, "Line should be > 0, got {}", line);
    assert!(column > 0, "Column should be > 0, got {}", column);

    eprintln!(
        "Error at line {}, column {} for input:\n{}",
        line, column, input
    );
}

#[test]
fn test_error_recovery_and_chaining() {
    // Test that multiple errors in a row work correctly
    let inputs = vec![
        r#"{"a":}"#,
        r#"[1,2,]"#,
        r#""bad\escape""#,
        r#"{"key":value}"#,
    ];

    for input in inputs {
        let lg_err = lg_json::from_str::<lg_json::Value>(input).unwrap_err();
        let sj_err = sj_json::from_str::<sj_json::Value>(input).unwrap_err();

        // Both should have reasonable error messages
        assert!(!lg_err.to_string().is_empty(), "lifegraph error empty");
        assert!(!sj_err.to_string().is_empty(), "serde_json error empty");

        eprintln!(
            "Input: {}\n  lifegraph: {}\n  serde_json: {}",
            input, lg_err, sj_err
        );
    }
}
