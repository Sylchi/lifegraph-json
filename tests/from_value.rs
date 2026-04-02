#![cfg(feature = "serde")]

extern crate serde_crate as serde;

use lifegraph_json::{from_value, json, to_value, Value};
use serde_crate::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(crate = "serde_crate")]
struct Payload {
    ok: bool,
    count: u64,
    tags: Vec<String>,
    mode: Mode,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(crate = "serde_crate")]
enum Mode {
    Fast,
    Slow { reason: String },
}

#[test]
fn from_value_deserializes_structs_and_enums_without_upstream_value_conversion() {
    let value = json!({
        "ok": true,
        "count": 7,
        "tags": ["a", "b"],
        "mode": { "Slow": { "reason": "compat" } }
    });

    let payload: Payload = from_value(value).unwrap();
    assert_eq!(
        payload,
        Payload {
            ok: true,
            count: 7,
            tags: vec!["a".into(), "b".into()],
            mode: Mode::Slow {
                reason: "compat".into(),
            },
        }
    );
}

#[test]
fn from_value_reports_type_mismatches() {
    let value = json!({
        "ok": true,
        "count": "wrong",
        "tags": [],
        "mode": "Fast"
    });

    let error = from_value::<Payload>(value).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("invalid type") || message.contains("invalid value"));
}

#[test]
fn to_value_and_from_value_roundtrip_structs() {
    let payload = Payload {
        ok: true,
        count: 11,
        tags: vec!["x".into()],
        mode: Mode::Fast,
    };

    let value: Value = to_value(&payload).unwrap();
    let reparsed: Payload = from_value(value).unwrap();
    assert_eq!(reparsed, payload);
}
