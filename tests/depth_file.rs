use lifegraph_json::{from_slice, JsonParseError};
use std::fs;

#[test]
fn test_100k_file() {
    let data =
        fs::read("tests/json_test_suite/test_parsing/n_structure_100000_opening_arrays.json")
            .expect("Failed to read test file");

    eprintln!("File size: {} bytes", data.len());
    let result = from_slice(&data);
    eprintln!("Result: {:?}", result);
}
