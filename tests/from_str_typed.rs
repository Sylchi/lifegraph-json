#![cfg(feature = "serde")]

extern crate serde_crate as serde;

use lifegraph_json::{from_reader, from_slice, from_str};
use serde_crate::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(crate = "serde_crate")]
struct Payload {
    ok: bool,
    count: u64,
    tags: Vec<String>,
}

#[test]
fn typed_from_str_uses_lifegraph_parser_path() {
    let payload: Payload =
        from_str(r#"{"ok":true,"count":7,"tags":["fast","compat"]}"#).unwrap();
    assert_eq!(
        payload,
        Payload {
            ok: true,
            count: 7,
            tags: vec!["fast".into(), "compat".into()],
        }
    );
}

#[test]
fn typed_from_slice_and_reader_work() {
    let payload: Payload =
        from_slice(br#"{"ok":false,"count":3,"tags":["slice"]}"#).unwrap();
    assert_eq!(payload.count, 3);

    let payload: Payload = from_reader(std::io::Cursor::new(
        br#"{"ok":true,"count":9,"tags":["reader"]}"# as &[u8],
    ))
    .unwrap();
    assert_eq!(payload.tags, vec!["reader"]);
}

#[test]
fn typed_from_str_reports_parse_errors() {
    let error = from_str::<Payload>(r#"{"ok":true,"count":7 trailing"#).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("unexpected") || message.contains("expected"));
}
