use lifegraph_json::parse_json;

#[test]
fn test_depth_500() {
    let mut json = vec![0u8; 127 * 2 + 1];
    for i in 0..127 {
        json[i] = b'[';
        json[127 * 2 - i] = b']';
    }
    json[127] = b'1';

    let json_str = std::str::from_utf8(&json).unwrap();
    let result = parse_json(json_str);
    eprintln!("127 depth: {:?}", result.is_ok());
    assert!(result.is_ok());
}

#[test]
fn test_depth_limit() {
    // Create 129 nested arrays - should hit the depth limit (max is 128)
    let mut json = vec![0u8; 129 * 2 + 1];
    for i in 0..129 {
        json[i] = b'[';
        json[129 * 2 - i] = b']';
    }
    json[129] = b'1';

    let json_str = std::str::from_utf8(&json).unwrap();
    let result = parse_json(json_str);
    eprintln!("129 depth: {:?}", result);
    // Should fail with NestingTooDeep
    assert!(
        result.is_err(),
        "Expected NestingTooDeep error, got: {:?}",
        result
    );
}
