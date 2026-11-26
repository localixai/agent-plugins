//! Port of `src/format/mcp-schema-check.ts`, switched.
//!
//! The check mcp.md ("Tools with invalid input schemas") names: "The schema must be valid against
//! the JSON Schema draft 2020-12 meta-schema", and "Top-level property names must be 1 to 64
//! characters long and use only ASCII letters and digits, `_`, `.`, and `-`". The crate has no
//! JSON Schema validator, so the meta-schema is carried as a table: each keyword of its
//! vocabularies (core, applicator, unevaluated, validation, format, content, meta-data, and the
//! deprecated `definitions` and `dependencies`) with the shape its value must have. A keyword the
//! vocabularies do not define is allowed, as in the meta-schema.

use std::collections::HashSet;

use crate::js::JsValue;

use crate::js::property_order;

type Report<'a> = &'a mut dyn FnMut(&str, &str);

/// How the value of a keyword is checked (the package's `Check` functions).
#[derive(Clone, Copy)]
enum Check {
    /// `typed('a string', ...)`.
    Text,
    /// `typed('a boolean', ...)`.
    Flag,
    /// Accepts any value.
    Anything,
    /// `typed('a number', ...)`.
    Number,
    /// `typed('a number above 0', ...)`.
    Positive,
    /// `typed('a non-negative integer', ...)`.
    Count,
    /// `typed('an array', ...)`.
    List,
    StringSet,
    TypeName,
    Schema,
    Schemas,
    SchemaOrStrings,
    /// `mapOf(each)`.
    MapOf(&'static Check),
}

const SIMPLE_TYPES: [&str; 7] = [
    "array", "boolean", "integer", "null", "number", "object", "string",
];

fn is_finite_number(value: &JsValue) -> bool {
    value.as_f64().is_some_and(f64::is_finite)
}

/// `typed(what, test)`: reports `must be <what>` when the test fails.
fn typed(what: &str, test: impl Fn(&JsValue) -> bool, value: &JsValue, at: &str, report: Report) {
    if !test(value) {
        report(at, &format!("must be {what}"));
    }
}

/// Whether the items are pairwise different as `JSON.stringify` writes them; both callers pass
/// only strings, whose text the wire's is.
fn unique(items: &[JsValue]) -> bool {
    let written: HashSet<String> = items.iter().map(JsValue::to_json_text).collect();
    written.len() == items.len()
}

fn string_set(value: &JsValue, at: &str, report: Report) {
    let fine = value
        .as_array()
        .is_some_and(|items| items.iter().all(JsValue::is_string) && unique(items));
    if !fine {
        report(at, "must be an array of unique strings");
    }
}

fn type_name(value: &JsValue, at: &str, report: Report) {
    let names: Vec<JsValue> = match value.as_array() {
        Some(items) => items.clone(),
        None => vec![value.clone()],
    };
    let fine = !names.is_empty()
        && names.iter().all(|name| {
            name.as_str()
                .is_some_and(|text| SIMPLE_TYPES.contains(&text))
        })
        && unique(&names);
    if !fine {
        report(
            at,
            &format!(
                "must be one of {}, or a list of unique ones",
                SIMPLE_TYPES.join(", ")
            ),
        );
    }
}

/// Keywords whose value is a schema and that the table below does not list: an object literal
/// member named `then` reads as a thenable to the linter.
const SCHEMA_BRANCHES: [&str; 1] = ["then"];

fn schema(value: &JsValue, at: &str, report: Report) {
    if value.is_boolean() {
        return;
    }
    let Some(members) = value.as_object() else {
        report(at, "must be a schema (an object or a boolean)");
        return;
    };
    for (keyword, inner) in property_order(members) {
        let check = keyword_check(keyword).or_else(|| {
            SCHEMA_BRANCHES
                .contains(&keyword.as_str())
                .then_some(Check::Schema)
        });
        if let Some(check) = check {
            run(check, inner, &format!("{at}/{keyword}"), report);
        }
    }
}

fn map_of(each: Check, value: &JsValue, at: &str, report: Report) {
    let Some(members) = value.as_object() else {
        report(at, "must be an object");
        return;
    };
    for (key, inner) in property_order(members) {
        run(each, inner, &format!("{at}/{key}"), report);
    }
}

fn schemas(value: &JsValue, at: &str, report: Report) {
    let Some(items) = value.as_array().filter(|items| !items.is_empty()) else {
        report(at, "must be a non-empty array of schemas");
        return;
    };
    for (index, inner) in items.iter().enumerate() {
        schema(inner, &format!("{at}/{index}"), report);
    }
}

fn schema_or_strings(value: &JsValue, at: &str, report: Report) {
    if value.is_array() {
        string_set(value, at, report);
    } else {
        schema(value, at, report);
    }
}

/// Applies one check to a value.
fn run(check: Check, value: &JsValue, at: &str, report: Report) {
    match check {
        Check::Text => typed("a string", JsValue::is_string, value, at, report),
        Check::Flag => typed("a boolean", JsValue::is_boolean, value, at, report),
        Check::Anything => {}
        Check::Number => typed("a number", is_finite_number, value, at, report),
        Check::Positive => typed(
            "a number above 0",
            |value| value.as_f64().is_some_and(|number| number > 0.0),
            value,
            at,
            report,
        ),
        Check::Count => typed(
            "a non-negative integer",
            |value| {
                value.as_f64().is_some_and(|number| {
                    number.is_finite() && number.fract() == 0.0 && number >= 0.0
                })
            },
            value,
            at,
            report,
        ),
        Check::List => typed("an array", JsValue::is_array, value, at, report),
        Check::StringSet => string_set(value, at, report),
        Check::TypeName => type_name(value, at, report),
        Check::Schema => schema(value, at, report),
        Check::Schemas => schemas(value, at, report),
        Check::SchemaOrStrings => schema_or_strings(value, at, report),
        Check::MapOf(each) => map_of(*each, value, at, report),
    }
}

/// The table of the meta-schema's keywords, in the package's order.
const KEYWORDS: &[(&str, Check)] = &[
    // core
    ("$id", Check::Text),
    ("$schema", Check::Text),
    ("$ref", Check::Text),
    ("$anchor", Check::Text),
    ("$dynamicRef", Check::Text),
    ("$dynamicAnchor", Check::Text),
    ("$vocabulary", Check::MapOf(&Check::Flag)),
    ("$comment", Check::Text),
    ("$defs", Check::MapOf(&Check::Schema)),
    ("definitions", Check::MapOf(&Check::Schema)),
    // applicator
    ("prefixItems", Check::Schemas),
    ("items", Check::Schema),
    ("contains", Check::Schema),
    ("additionalProperties", Check::Schema),
    ("properties", Check::MapOf(&Check::Schema)),
    ("patternProperties", Check::MapOf(&Check::Schema)),
    ("dependentSchemas", Check::MapOf(&Check::Schema)),
    ("dependencies", Check::MapOf(&Check::SchemaOrStrings)),
    ("propertyNames", Check::Schema),
    ("if", Check::Schema),
    ("else", Check::Schema),
    ("allOf", Check::Schemas),
    ("anyOf", Check::Schemas),
    ("oneOf", Check::Schemas),
    ("not", Check::Schema),
    // unevaluated
    ("unevaluatedItems", Check::Schema),
    ("unevaluatedProperties", Check::Schema),
    // validation
    ("type", Check::TypeName),
    ("const", Check::Anything),
    ("enum", Check::List),
    ("multipleOf", Check::Positive),
    ("maximum", Check::Number),
    ("exclusiveMaximum", Check::Number),
    ("minimum", Check::Number),
    ("exclusiveMinimum", Check::Number),
    ("maxLength", Check::Count),
    ("minLength", Check::Count),
    ("pattern", Check::Text),
    ("maxItems", Check::Count),
    ("minItems", Check::Count),
    ("uniqueItems", Check::Flag),
    ("maxContains", Check::Count),
    ("minContains", Check::Count),
    ("maxProperties", Check::Count),
    ("minProperties", Check::Count),
    ("required", Check::StringSet),
    ("dependentRequired", Check::MapOf(&Check::StringSet)),
    // format, content and meta-data
    ("format", Check::Text),
    ("contentEncoding", Check::Text),
    ("contentMediaType", Check::Text),
    ("contentSchema", Check::Schema),
    ("title", Check::Text),
    ("description", Check::Text),
    ("default", Check::Anything),
    ("deprecated", Check::Flag),
    ("readOnly", Check::Flag),
    ("writeOnly", Check::Flag),
    ("examples", Check::List),
];

/// The check of a keyword, `Object.hasOwn(KEYWORDS, keyword)` and its value.
fn keyword_check(keyword: &str) -> Option<Check> {
    KEYWORDS
        .iter()
        .find(|(name, _)| *name == keyword)
        .map(|(_, check)| *check)
}

/// The places in `value` that break the draft 2020-12 meta-schema, as `path: message`.
#[must_use]
pub fn meta_schema_problems(value: &JsValue) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    schema(value, "", &mut |path, message| {
        let shown = if path.is_empty() { "(root)" } else { path };
        found.push(format!("{shown} {message}"));
    });
    found
}

/// "1 to 64 characters long and use only ASCII letters and digits, `_`, `.`, and `-`" (the API's
/// `^[a-zA-Z0-9_.-]{1,64}$`).
fn is_valid_property_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
}

/// The top-level properties whose names the API would refuse.
#[must_use]
pub fn property_name_problems(value: &JsValue) -> Vec<String> {
    let Some(properties) = value.get("properties").and_then(JsValue::as_object) else {
        return Vec::new();
    };
    property_order(properties)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| !is_valid_property_name(name))
        .map(|name| {
            format!("\"{name}\" is not a valid property name: use 1 to 64 ASCII letters, digits, _, . or -")
        })
        .collect()
}

/// The dialect the meta-schema check applies to: none declared, or draft 2020-12.
const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// Whether the meta-schema check applies to `value`'s declared dialect.
#[must_use]
pub fn checks_meta_schema(value: &JsValue) -> bool {
    match value.get("$schema").and_then(JsValue::as_str) {
        None => true,
        Some(declared) => declared.strip_suffix('#').unwrap_or(declared) == DRAFT_2020_12,
    }
}

crate::registry::exports! {
    "format/mcp-schema-check" {
        "metaSchemaProblems" => ok |value: JsValue| meta_schema_problems(&value);
        "propertyNameProblems" => ok |value: JsValue| property_name_problems(&value);
        "checksMetaSchema" => ok |value: JsValue| checks_meta_schema(&value);
    }
}
