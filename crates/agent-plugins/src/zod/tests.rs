//! What the oracle (`tests/differential/js/zod.rs`) cannot show: a schema shared and cloned,
//! the patterns the package writes compiling, the error's text, and the issue accessors.

use super::*;
use crate::js::JsValue;

fn object_input<const N: usize>(members: [(&str, JsValue); N]) -> JsValue {
    JsValue::from(members)
}

#[test]
fn a_shared_schema_answers_the_same_from_every_clone() {
    let schema = object(fields! { "name" => string() });
    let copy = schema.clone();
    let input = object_input([("name", "a".into()), ("extra", true.into())]);
    assert_eq!(schema.safe_parse(&input), copy.safe_parse(&input));
    assert_eq!(
        schema.safe_parse(&input),
        Ok(object_input([("name", "a".into())]))
    );
}

#[test]
fn every_pattern_the_package_writes_compiles() {
    for source in [r"^\.", "^[A-Za-z_][A-Za-z0-9_]*$", "^[a-z0-9.-]*$"] {
        let (literal, regex) = build::pattern(source, "");
        assert!(regex.is_some(), "{literal}");
    }
}

#[test]
fn a_pattern_rust_cannot_read_refuses_every_value() {
    let schema = string().regex("(?<=a)b");
    assert!(schema.safe_parse(&"ab".into()).is_err());
}

#[test]
fn the_error_text_is_the_issues_as_json_stringify_writes_them() {
    let failure = string().safe_parse(&1.0.into()).unwrap_err();
    let text = failure.to_string();
    assert!(
        text.starts_with("[\n  {\n    \"expected\": \"string\","),
        "{text}"
    );
    assert!(text.contains("\"message\": \"Invalid input: expected string, received number\""));
}

#[test]
fn an_issue_reads_its_code_path_and_message() {
    let schema = object(fields! { "list" => array(string()) });
    let input = object_input([("list", JsValue::Array(vec![1.0.into()]))]);
    let failure = schema.safe_parse(&input).unwrap_err();
    let [found] = failure.issues.as_slice() else {
        panic!("one issue");
    };
    assert_eq!(found.code(), "invalid_type");
    assert_eq!(found.path(), &[JsValue::from("list"), JsValue::Number(0.0)]);
    assert_eq!(
        found.message(),
        "Invalid input: expected string, received number"
    );
}

#[test]
fn a_spread_keeps_the_first_place_and_the_last_schema() {
    let common = fields! { "a" => string(), "b" => string() };
    let shape = fields! { "b" => number(), ..common, "c" => boolean() };
    let keys: Vec<&String> = shape.keys().collect();
    assert_eq!(keys, ["b", "a", "c"]);
    assert!(matches!(
        shape.get("b").map(Schema::kind),
        Some(schema::Kind::String)
    ));
}

#[test]
fn a_lazy_schema_holds_itself() {
    let nested = JsValue::Array(vec![JsValue::Array(vec![1.0.into(), "x".into()])]);
    assert_eq!(json().safe_parse(&nested), Ok(nested.clone()));
}
