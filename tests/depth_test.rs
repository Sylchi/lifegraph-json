use lifegraph_json::from_slice;
use std::fs;

#[test]
fn test_depth_limiting_file() {
    // Read the 100k nesting file
    let data =
        fs::read("tests/json_test_suite/test_parsing/n_structure_100000_opening_arrays.json")
            .expect("Failed to read test file");

    let result = from_slice(&data);
    eprintln!("100k depth file result: {:?}", result);

    // Should fail with NestingTooDeep, not stack overflow
    assert!(
        format!("{:?}", result).contains("NestingTooDeep") || result.is_err(),
        "Expected error (preferably NestingTooDeep), got: {:?}",
        result
    );
}

#[test]
fn test_500_nesting() {
    // 500 nested arrays should work fine
    let mut json = Vec::new();
    for _ in 0..500 {
        json.push(b'[');
    }
    json.push(b'1');
    for _ in 0..500 {
        json.push(b']');
    }

    let result = from_slice(&json);
    eprintln!(
        "500 depth: {:?}",
        result.as_ref().map(|_| "ok").unwrap_or("err")
    );
    assert!(result.is_ok(), "500 nesting should work");
}
