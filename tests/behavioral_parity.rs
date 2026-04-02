#![cfg(all(feature = "serde", feature = "raw_value"))]

use serde_json as local_json;
use serde_json_upstream as upstream_json;

#[test]
fn raw_value_top_level_borrowed_deserialization_matches_upstream() {
    let input = r#"{"nested":[1,true,{"x":"y"}]}"#;

    let local: &local_json::RawValue = local_json::from_str(input).unwrap();
    let upstream: &upstream_json::value::RawValue = upstream_json::from_str(input).unwrap();

    assert_eq!(local.get(), upstream.get());
}

#[test]
fn raw_value_top_level_boxed_deserialization_matches_upstream() {
    let input = r#"[null,{"a":1},"two"]"#;

    let local: Box<local_json::RawValue> = local_json::from_str(input).unwrap();
    let upstream: Box<upstream_json::value::RawValue> = upstream_json::from_str(input).unwrap();

    assert_eq!(local.get(), upstream.get());
}

#[test]
fn raw_value_nested_borrowed_deserialization_matches_upstream() {
    #[derive(Debug, serde_crate::Deserialize)]
    #[serde(crate = "serde_crate")]
    struct Msg<'a> {
        id: u32,
        #[serde(borrow)]
        payload: &'a local_json::RawValue,
    }

    #[derive(Debug, serde_crate::Deserialize)]
    #[serde(crate = "serde_crate")]
    struct UpstreamMsg<'a> {
        id: u32,
        #[serde(borrow)]
        payload: &'a upstream_json::value::RawValue,
    }

    let input = r#"{"id":1,"payload":{"nested":[1,true]}}"#;

    let local: Msg<'_> = local_json::from_str(input).unwrap();
    let upstream: UpstreamMsg<'_> = upstream_json::from_str(input).unwrap();

    assert_eq!(local.id, upstream.id);
    assert_eq!(local.payload.get(), upstream.payload.get());
}

#[test]
fn raw_value_nested_boxed_deserialization_matches_upstream() {
    #[derive(Debug, serde_crate::Deserialize)]
    #[serde(crate = "serde_crate")]
    struct Msg {
        id: u32,
        payload: Box<local_json::RawValue>,
    }

    #[derive(Debug, serde_crate::Deserialize)]
    #[serde(crate = "serde_crate")]
    struct UpstreamMsg {
        id: u32,
        payload: Box<upstream_json::value::RawValue>,
    }

    let input = r#"{"id":1,"payload":{"nested":[1,true]}}"#;

    let local: Msg = local_json::from_str(input).unwrap();
    let upstream: UpstreamMsg = upstream_json::from_str(input).unwrap();

    assert_eq!(local.id, upstream.id);
    assert_eq!(local.payload.get(), upstream.payload.get());
}

#[test]
fn raw_value_enum_newtype_variant_matches_upstream() {
    #[derive(Debug, serde_crate::Deserialize)]
    #[serde(crate = "serde_crate")]
    enum Msg<'a> {
        Data(#[serde(borrow)] &'a local_json::RawValue),
    }

    #[derive(Debug, serde_crate::Deserialize)]
    #[serde(crate = "serde_crate")]
    enum UpstreamMsg<'a> {
        Data(#[serde(borrow)] &'a upstream_json::value::RawValue),
    }

    let input = r#"{"Data":{"nested":[1,true]}}"#;

    let local: Msg<'_> = local_json::from_str(input).unwrap();
    let upstream: UpstreamMsg<'_> = upstream_json::from_str(input).unwrap();

    match (local, upstream) {
        (Msg::Data(local), UpstreamMsg::Data(upstream)) => {
            assert_eq!(local.get(), upstream.get());
        }
    }
}

#[test]
fn raw_value_enum_struct_variant_matches_upstream() {
    #[derive(Debug, serde_crate::Deserialize)]
    #[serde(crate = "serde_crate")]
    enum Msg<'a> {
        Data {
            id: u32,
            #[serde(borrow)]
            payload: &'a local_json::RawValue,
        },
    }

    #[derive(Debug, serde_crate::Deserialize)]
    #[serde(crate = "serde_crate")]
    enum UpstreamMsg<'a> {
        Data {
            id: u32,
            #[serde(borrow)]
            payload: &'a upstream_json::value::RawValue,
        },
    }

    let input = r#"{"Data":{"id":1,"payload":{"nested":[1,true]}}}"#;

    let local: Msg<'_> = local_json::from_str(input).unwrap();
    let upstream: UpstreamMsg<'_> = upstream_json::from_str(input).unwrap();

    match (local, upstream) {
        (
            Msg::Data { id: local_id, payload: local_payload },
            UpstreamMsg::Data { id: upstream_id, payload: upstream_payload },
        ) => {
            assert_eq!(local_id, upstream_id);
            assert_eq!(local_payload.get(), upstream_payload.get());
        }
    }
}

#[test]
fn to_raw_value_matches_upstream_rendering() {
    #[derive(serde_crate::Serialize)]
    #[serde(crate = "serde_crate")]
    struct Payload<'a> {
        name: &'a str,
        ok: bool,
        count: u32,
    }

    let payload = Payload {
        name: "node-1",
        ok: true,
        count: 7,
    };

    let local = local_json::to_raw_value(&payload).unwrap();
    let upstream = upstream_json::value::to_raw_value(&payload).unwrap();

    assert_eq!(local.get(), upstream.get());
}

#[test]
fn parse_error_shape_matches_upstream_for_selected_cases() {
    let local = local_json::from_str::<local_json::Value>("[1,]").unwrap_err();
    let upstream = upstream_json::from_str::<upstream_json::Value>("[1,]").unwrap_err();

    assert_eq!(local.classify(), map_category(upstream.classify()));
    assert_eq!(local.line(), upstream.line());
    assert_eq!(local.column(), upstream.column());

    let local = local_json::from_str::<local_json::Value>("\"\\uD83C\\uFFFF\"").unwrap_err();
    let upstream = upstream_json::from_str::<upstream_json::Value>("\"\\uD83C\\uFFFF\"").unwrap_err();

    assert_eq!(local.classify(), map_category(upstream.classify()));
    assert_eq!(local.line(), upstream.line());
}

fn map_category(category: upstream_json::error::Category) -> local_json::Category {
    match category {
        upstream_json::error::Category::Io => local_json::Category::Io,
        upstream_json::error::Category::Syntax => local_json::Category::Syntax,
        upstream_json::error::Category::Data => local_json::Category::Data,
        upstream_json::error::Category::Eof => local_json::Category::Eof,
    }
}

#[test]
fn preserve_order_parse_matches_upstream() {
    let input = r#"{"z":0,"a":1,"m":2}"#;

    let local: local_json::Value = local_json::from_str(input).unwrap();
    let upstream: upstream_json::Value = upstream_json::from_str(input).unwrap();

    let local_keys: Vec<_> = local.as_object().unwrap().keys().cloned().collect();
    let upstream_keys: Vec<_> = upstream.as_object().unwrap().keys().cloned().collect();

    assert_eq!(local_keys, upstream_keys);
}

#[test]
fn preserve_order_map_mutations_match_upstream() {
    let mut local = local_json::Map::new();
    local.insert("a".into(), 1.into());
    local.insert("b".into(), 2.into());
    local.insert("c".into(), 3.into());

    let mut upstream = upstream_json::Map::new();
    upstream.insert("a".into(), 1.into());
    upstream.insert("b".into(), 2.into());
    upstream.insert("c".into(), 3.into());

    assert_eq!(local.shift_insert(0, "c".into(), 9.into()).map(value_to_string), upstream.shift_insert(0, "c".into(), 9.into()).map(value_to_string_upstream));
    assert_eq!(keys(&local), keys_upstream(&upstream));

    assert_eq!(local.swap_remove("a").map(value_to_string), upstream.swap_remove("a").map(value_to_string_upstream));
    assert_eq!(keys(&local), keys_upstream(&upstream));

    local.sort_keys();
    upstream.sort_keys();
    assert_eq!(keys(&local), keys_upstream(&upstream));
}

#[test]
fn serializer_output_matches_upstream_for_value_corpus() {
    let cases = [
        r#"null"#,
        r#"true"#,
        r#"123"#,
        r#""hello\nworld""#,
        r#"[1,true,{"a":[null,"x"]}]"#,
        r#"{"z":0,"a":[1,2,3],"nested":{"ok":true,"msg":"hi"}}"#,
    ];

    for input in cases {
        let local: local_json::Value = local_json::from_str(input).unwrap();
        let upstream: upstream_json::Value = upstream_json::from_str(input).unwrap();

        assert_eq!(local_json::to_string(&local).unwrap(), upstream_json::to_string(&upstream).unwrap(), "input={input}");
        assert_eq!(
            local_json::to_string_pretty(&local).unwrap(),
            upstream_json::to_string_pretty(&upstream).unwrap(),
            "input={input}"
        );
    }
}

#[test]
fn parse_roundtrip_matches_upstream_for_selected_inputs() {
    let cases = [
        r#"{"a":1,"b":[true,false,null],"c":{"x":"y"}}"#,
        r#"[{"id":1},{"id":2,"tags":["a","b"]}]"#,
        r#"{"unicode":"\u2603","escaped":"line\nbreak","num":-12.5}"#,
    ];

    for input in cases {
        let local: local_json::Value = local_json::from_str(input).unwrap();
        let upstream: upstream_json::Value = upstream_json::from_str(input).unwrap();

        assert_eq!(local_json::to_string(&local).unwrap(), upstream_json::to_string(&upstream).unwrap(), "input={input}");
    }
}

fn keys(map: &local_json::Map) -> Vec<String> {
    map.keys().cloned().collect()
}

fn keys_upstream(map: &upstream_json::Map<String, upstream_json::Value>) -> Vec<String> {
    map.keys().cloned().collect()
}

fn value_to_string(value: local_json::Value) -> String {
    local_json::to_string(&value).unwrap()
}

fn value_to_string_upstream(value: upstream_json::Value) -> String {
    upstream_json::to_string(&value).unwrap()
}
