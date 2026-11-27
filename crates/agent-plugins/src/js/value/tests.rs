use serde::{Deserialize, Serialize};

use super::{JsObject, JsValue, number_tag, to_js_value};
use crate::js::json_parse::json_parse;

/// The text as the wire reads it: parsed as JSON, then its tags read back.
fn read(text: &str) -> JsValue {
    JsValue::deserialize(json_parse(text).unwrap_or_default()).unwrap_or_default()
}

#[test]
fn the_four_numbers_json_lacks_are_tagged_both_ways() {
    let cases = [
        (f64::NAN, r#"{"$number":"NaN"}"#),
        (f64::INFINITY, r#"{"$number":"Infinity"}"#),
        (f64::NEG_INFINITY, r#"{"$number":"-Infinity"}"#),
        (-0.0, r#"{"$number":"-0"}"#),
    ];
    for (number, text) in cases {
        assert_eq!(JsValue::Number(number).to_json_text(), text);
        let back = read(text).as_f64().unwrap_or_default();
        assert_eq!(number_tag(back), number_tag(number), "{text}");
    }
    assert_eq!(number_tag(0.0), None);
    assert_eq!(number_tag(1.5), None);
}

#[test]
fn negative_zero_round_trips_and_zero_stays_plain() {
    let back = read(&JsValue::Number(-0.0).to_json_text());
    assert!(
        back.as_f64()
            .is_some_and(|zero| zero == 0.0 && zero.is_sign_negative())
    );
    assert_eq!(JsValue::Number(0.0).to_json_text(), "0");
    let parsed = json_parse("-0").unwrap_or_default();
    assert!(parsed.as_f64().is_some_and(f64::is_sign_negative));
}

#[test]
fn undefined_is_tagged_both_ways() {
    assert_eq!(JsValue::Undefined.to_json_text(), r#"{"$undefined":true}"#);
    assert_eq!(read(r#"{"$undefined":true}"#), JsValue::Undefined);
    assert_eq!(read(r#"{"$undefined":false}"#).type_of(), "object");
    assert_eq!(read(r#"{"$number":"1"}"#).type_of(), "object");
}

#[test]
fn nan_is_kept_as_an_array_item_and_an_object_member() {
    let text = r#"[1,{"$number":"NaN"},{"a":{"$number":"NaN"},"b":{"$undefined":true}}]"#;
    let value = read(text);
    let items = value.as_array().cloned().unwrap_or_default();
    assert!(
        items
            .get(1)
            .and_then(JsValue::as_f64)
            .is_some_and(f64::is_nan)
    );
    let member = items.get(2).and_then(|object| object.get("a"));
    assert!(member.and_then(JsValue::as_f64).is_some_and(f64::is_nan));
    assert_eq!(value.to_json_text(), text);
}

#[test]
fn undefined_is_kept_as_an_object_member() {
    let value = read(r#"{"a":{"$undefined":true},"b":null}"#);
    assert_eq!(value.get("a"), Some(&JsValue::Undefined));
    assert_eq!(value.get("b"), Some(&JsValue::Null));
    assert_eq!(value.get("c"), None);
    assert_eq!(
        value.to_json_text(),
        r#"{"a":{"$undefined":true},"b":null}"#
    );
}

// bun -e 'console.log(JSON.stringify([1,-2,1.5,1e21,0.000001,1e-7,2**60]))'
#[test]
fn numbers_are_written_as_javascript_writes_them() {
    let items: Vec<JsValue> = [1.0, -2.0, 1.5, 1e21, 0.000_001, 1e-7, 2f64.powi(60)]
        .into_iter()
        .map(JsValue::Number)
        .collect();
    assert_eq!(
        JsValue::Array(items).to_json_text(),
        "[1,-2,1.5,1e+21,0.000001,1e-7,1152921504606847000]"
    );
    assert_eq!(
        JsValue::from("a\"\\\n\u{1}\u{2028}é").to_json_text(),
        "\"a\\\"\\\\\\n\\u0001\u{2028}é\""
    );
}

#[test]
fn type_of_is_javascripts() {
    let cases = [
        (JsValue::Undefined, "undefined"),
        (JsValue::Null, "object"),
        (JsValue::Bool(true), "boolean"),
        (JsValue::Number(f64::NAN), "number"),
        (JsValue::from("x"), "string"),
        (JsValue::Array(Vec::new()), "object"),
        (JsValue::Object(JsObject::new()), "object"),
    ];
    for (value, name) in cases {
        assert_eq!(value.type_of(), name);
    }
    assert!(JsValue::Undefined.is_nullish() && JsValue::Null.is_nullish());
    assert!(!JsValue::Bool(false).is_nullish());
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Bounds {
    min: Option<f64>,
    max: Option<f64>,
    count: Option<u32>,
}

#[test]
fn a_typed_parameter_decodes_from_a_value() {
    let value = read(r#"{"min":{"$number":"-Infinity"},"max":{"$undefined":true},"count":3}"#);
    let bounds = Bounds::deserialize(value).ok();
    assert_eq!(
        bounds,
        Some(Bounds {
            min: Some(f64::NEG_INFINITY),
            max: None,
            count: Some(3),
        })
    );
    assert!(String::deserialize(JsValue::Undefined).is_err());
    assert_eq!(
        Option::<String>::deserialize(JsValue::Null).ok(),
        Some(None)
    );
}

#[test]
fn a_serializable_value_becomes_a_value() {
    let bounds = Bounds {
        min: Some(f64::NAN),
        max: None,
        count: Some(2),
    };
    let value = to_js_value(&bounds).unwrap_or_default();
    assert!(
        value
            .get("min")
            .and_then(JsValue::as_f64)
            .is_some_and(f64::is_nan)
    );
    assert_eq!(value.get("max"), Some(&JsValue::Null));
    assert_eq!(
        value.to_json_text(),
        r#"{"min":{"$number":"NaN"},"max":null,"count":2}"#
    );
    let items = vec![JsValue::Undefined, JsValue::Number(-0.0)];
    let written = to_js_value(&items).ok();
    assert_eq!(written, Some(JsValue::Array(items)));
}

#[test]
fn a_value_the_host_keeps_is_no_struct_of_data() {
    #[derive(Debug, Deserialize)]
    struct Facts {
        #[serde(default)]
        flag: Option<bool>,
    }
    let port = JsValue::from([("$port", JsValue::from(3.0))]);
    let refused = Facts::deserialize(port).map(|facts| facts.flag);
    assert!(refused.is_err_and(|error| error.to_string().contains("Facts is data")));
    let data = JsValue::from([("flag", JsValue::Bool(true))]);
    assert_eq!(
        Facts::deserialize(data).map(|facts| facts.flag).ok(),
        Some(Some(true))
    );
    let kept = crate::registry::Port::deserialize(JsValue::from([("$port", JsValue::from(3.0))]));
    assert!(kept.is_ok());
}
