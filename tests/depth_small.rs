use lifegraph_json::from_slice;

#[test]
fn test_depth_500() {
    let mut json = vec![0u8; 500 * 2 + 1];
    for i in 0..500 {
        json[i] = b'[';
        json[500 * 2 - i] = b']';
    }
    json[500] = b'1';

    let result = from_slice(&json);
    eprintln!("500 depth: {:?}", result.is_ok());
    assert!(result.is_ok());
}

#[test]
fn test_depth_limit() {
    // Create 1001 nested arrays - should hit the depth limit (max is 1000)
    let mut json = vec![0u8; 1001 * 2 + 1];
    for i in 0..1001 {
        json[i] = b'[';
        json[1001 * 2 - i] = b']';
    }
    json[1001] = b'1';

    let result = from_slice(&json);
    eprintln!("1001 depth: {:?}", result);
    // Should fail with NestingTooDeep
    assert!(format!("{:?}", result).contains("NestingTooDeep"));
}
