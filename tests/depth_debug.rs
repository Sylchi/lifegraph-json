use lifegraph_json::{from_slice, JsonParseError};

#[test]
fn test_debug_depth() {
    // Test with increasing depths to find where it fails
    for depth in [100, 500, 1000, 5000, 10000, 15000] {
        // Use Box to allocate on heap, not stack
        let mut json = vec![0u8; depth * 2 + 1];
        for i in 0..depth {
            json[i] = b'[';
            json[depth * 2 - i] = b']';
        }
        json[depth] = b'1';

        let result = from_slice(&json);
        match &result {
            Ok(_) => eprintln!("Depth {}: OK", depth),
            Err(JsonParseError::NestingTooDeep { depth: d, max }) => {
                eprintln!("Depth {}: NestingTooDeep at {} (max {})", depth, d, max);
            }
            Err(e) => eprintln!("Depth {}: Error: {:?}", depth, e),
        }

        if result.is_err() {
            break;
        }
    }
}
