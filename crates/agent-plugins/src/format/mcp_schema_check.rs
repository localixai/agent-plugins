//! Ported from `src/format/mcp-schema-check.ts`; only the core uses it now, so it has no wrapper.
//!
//! The check mcp.md ("Tools with invalid input schemas") names: "The schema must be valid against
//! the JSON Schema draft 2020-12 meta-schema", and "Top-level property names must be 1 to 64
//! characters long and use only ASCII letters and digits, `_`, `.`, and `-`". The meta-schema is
//! the official one, checked by `flowschema` (docs/RUST.md, RS8): its root and the documents of its
//! seven vocabularies, in `mcp_schema_check/` as json-schema.org serves them
//! (`https://json-schema.org/draft/2020-12/schema` and `.../draft/2020-12/meta/<name>`, fetched
//! 2026-11-27). A keyword the vocabularies do not define is allowed, as in the meta-schema.
//!
//! Each document is resolved in two steps before it is compiled (`resolve`), so that flowschema
//! gives the meta-schema's own verdict:
//!
//! - `"$dynamicRef": "#meta"` is a `$ref` to the root. A check starts at the root, whose
//!   `$dynamicAnchor` is the outermost `meta` of every dynamic scope, so each one lands there;
//!   flowschema does not follow a `$dynamicRef` across resources (its `unchecked` names it), which
//!   would check a nested schema by the one vocabulary that refers to it.
//! - A `format` is taken out. The root declares the format-annotation vocabulary, under which
//!   `format` asserts nothing (JSON Schema Validation 2020-12, 7.2.1); flowschema asserts every
//!   format, which would refuse a valid `"$ref": "a b"` or `"pattern": "("`.

use std::sync::LazyLock;

use flowschema::Schema;

use crate::js::json_parse::json_parse;
use crate::js::json_value::json_value;
use crate::js::{JsValue, property_order};

/// The meta-schema's root, as the dialect names it.
const ROOT: &str = include_str!("mcp_schema_check/schema.json");

/// The vocabularies' documents, each with the URI the root's `$ref`s resolve to.
const VOCABULARIES: [(&str, &str); 7] = [
    (
        "https://json-schema.org/draft/2020-12/meta/core",
        include_str!("mcp_schema_check/meta/core.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/applicator",
        include_str!("mcp_schema_check/meta/applicator.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/unevaluated",
        include_str!("mcp_schema_check/meta/unevaluated.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/validation",
        include_str!("mcp_schema_check/meta/validation.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/meta-data",
        include_str!("mcp_schema_check/meta/meta-data.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/format-annotation",
        include_str!("mcp_schema_check/meta/format-annotation.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/content",
        include_str!("mcp_schema_check/meta/content.json"),
    ),
];

/// The two steps of the module comment, at every depth of a document.
fn resolve(node: &mut JsValue) {
    match node {
        JsValue::Object(members) => {
            if members.get("$dynamicRef").and_then(JsValue::as_str) == Some("#meta") {
                members.shift_remove("$dynamicRef");
                members.insert("$ref".to_owned(), DRAFT_2020_12.into());
            }
            if members.get("format").is_some_and(JsValue::is_string) {
                members.shift_remove("format");
            }
            members.values_mut().for_each(resolve);
        }
        JsValue::Array(items) => items.iter_mut().for_each(resolve),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_) => {}
    }
}

/// A document read and resolved; one that does not read is `false`, which refuses every schema
/// rather than passing them unchecked (a test reads each).
fn document(text: &str) -> JsValue {
    let mut read = json_parse(text).unwrap_or(JsValue::Bool(false));
    resolve(&mut read);
    read
}

/// The meta-schema, compiled once.
static META_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    let vocabularies: Vec<_> = VOCABULARIES
        .iter()
        .map(|(uri, text)| {
            (
                (*uri).to_owned(),
                json_value(&document(text)).unwrap_or_default(),
            )
        })
        .collect();
    Schema::with_documents(
        &json_value(&document(ROOT)).unwrap_or_default(),
        &vocabularies,
    )
});

/// The places in `value` that break the draft 2020-12 meta-schema, each once, as flowschema words
/// them with the place named from the tool's `inputSchema` (`inputSchema.properties.a.type
/// matches none of the 2 schemas of anyOf`).
#[must_use]
pub fn meta_schema_problems(value: &JsValue) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for problem in META_SCHEMA.mismatches(&json_value(value).unwrap_or_default()) {
        let shown = problem
            .strip_prefix("data")
            .map_or_else(|| problem.clone(), |place| format!("inputSchema{place}"));
        if !found.contains(&shown) {
            found.push(shown);
        }
    }
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

#[cfg(test)]
mod tests;
