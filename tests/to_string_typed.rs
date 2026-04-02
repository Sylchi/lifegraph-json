#![cfg(feature = "serde")]

extern crate serde_crate as serde;

use serde_json::{
    to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer, to_writer_pretty,
};
use serde_crate::Serialize;

#[derive(Serialize)]
#[serde(crate = "serde_crate")]
struct Payload<'a> {
    ok: bool,
    count: u64,
    msg: &'a str,
}

#[derive(Serialize)]
#[serde(crate = "serde_crate")]
struct NonFinite {
    value: f64,
}

#[test]
fn typed_serialization_preserves_struct_field_order() {
    let payload = Payload {
        ok: true,
        count: 7,
        msg: "hello",
    };

    assert_eq!(
        to_string(&payload).unwrap(),
        r#"{"ok":true,"count":7,"msg":"hello"}"#
    );
    assert_eq!(
        String::from_utf8(to_vec(&payload).unwrap()).unwrap(),
        r#"{"ok":true,"count":7,"msg":"hello"}"#
    );
}

#[test]
fn typed_writer_and_pretty_serialization_use_local_emitters() {
    let payload = Payload {
        ok: false,
        count: 3,
        msg: "writer",
    };

    let pretty = to_string_pretty(&payload).unwrap();
    assert!(pretty.contains("\"ok\": false"));
    assert!(pretty.contains("\"count\": 3"));

    let mut compact = Vec::new();
    to_writer(&mut compact, &payload).unwrap();
    assert_eq!(
        String::from_utf8(compact).unwrap(),
        r#"{"ok":false,"count":3,"msg":"writer"}"#
    );

    let mut pretty_bytes = Vec::new();
    to_writer_pretty(&mut pretty_bytes, &payload).unwrap();
    assert_eq!(String::from_utf8(pretty_bytes).unwrap(), pretty);

    let pretty_vec = String::from_utf8(to_vec_pretty(&payload).unwrap()).unwrap();
    assert_eq!(pretty_vec, pretty);
}

#[test]
fn typed_serialization_rejects_non_finite_float() {
    let error = to_string(&NonFinite { value: f64::NAN }).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("non-finite") || message.contains("finite"));
}
