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
fn test_depth_10000() {
    let mut json = vec![0u8; 10000 * 2 + 1];
    for i in 0..10000 {
        json[i] = b'[';
        json[10000 * 2 - i] = b']';
    }
    json[10000] = b'1';

    let result = from_slice(&json);
    eprintln!("10000 depth: {:?}", result);
    // Should fail with NestingTooDeep
    assert!(format!("{:?}", result).contains("NestingTooDeep"));
}
