//! The meta-schema check: the official documents read whole and checked by flowschema with nothing
//! left unchecked, and the verdicts where it differs from the table it replaced.

use super::{META_SCHEMA, ROOT, VOCABULARIES, meta_schema_problems};
use crate::js::JsValue;
use crate::js::json_parse::json_parse;

fn schema(text: &str) -> JsValue {
    json_parse(text).unwrap()
}

fn problems(text: &str) -> Vec<String> {
    meta_schema_problems(&schema(text))
}

#[test]
fn every_document_reads_and_nothing_is_left_unchecked() {
    assert!(json_parse(ROOT).unwrap().is_object());
    for (uri, text) in VOCABULARIES {
        let read = json_parse(text).unwrap();
        assert_eq!(read.get("$id").and_then(JsValue::as_str), Some(uri));
    }
    assert!(META_SCHEMA.unchecked().is_empty());
}

#[test]
fn a_nested_schema_is_checked_by_every_vocabulary() {
    assert_eq!(
        problems(r#"{"properties":{"a":{"properties":{"b":{"minLength":-1}}}}}"#),
        ["inputSchema.properties.a.properties.b.minLength must be >= 0 (is -1)"]
    );
    assert_eq!(
        problems(r#"{"properties":{"a":{"type":"nope"}}}"#),
        ["inputSchema.properties.a.type matches none of the 2 schemas of anyOf"]
    );
}

#[test]
fn each_problem_is_said_once() {
    assert_eq!(
        problems(r#"{"$defs":{"a":5}}"#),
        ["inputSchema.$defs.a must be object or boolean (is integer)"]
    );
    assert_eq!(
        meta_schema_problems(&JsValue::Undefined),
        ["inputSchema must be object or boolean (is null)"]
    );
}

#[test]
fn a_format_is_an_annotation_only() {
    for text in [
        r#"{"$ref":"a b"}"#,
        r#"{"properties":{"a":{"pattern":"("}}}"#,
        r#"{"$id":"::"}"#,
        r#"{"$schema":"not a uri"}"#,
        r#"{"$vocabulary":{"not a uri":true}}"#,
    ] {
        assert_eq!(problems(text), Vec::<String>::new(), "{text}");
    }
}

/// Where the meta-schema refuses what the table took: an anchor's name, a fragment in `$id`, and
/// the deprecated `$recursiveAnchor`.
#[test]
fn the_meta_schemas_own_patterns_hold() {
    for text in [
        r#"{"$anchor":"1x"}"#,
        r#"{"$dynamicAnchor":"1"}"#,
        r#"{"$id":"https://example.com/s#frag"}"#,
        r#"{"$recursiveAnchor":5}"#,
    ] {
        assert_eq!(problems(text).len(), 1, "{text}");
    }
}

/// A number with no JSON text is `null` as the API receives it, which no number keyword takes.
#[test]
fn a_number_json_cannot_write_is_null() {
    for number in [f64::NAN, f64::INFINITY] {
        let value = JsValue::from([("multipleOf", JsValue::Number(number))]);
        assert_eq!(
            meta_schema_problems(&value),
            ["inputSchema.multipleOf must be number (is null)"]
        );
    }
    let fine = JsValue::from([
        ("minimum", JsValue::Number(-0.0)),
        ("minLength", 2.0.into()),
    ]);
    assert!(meta_schema_problems(&fine).is_empty());
}
