//! Port of `src/format/core/extensions.ts`, switched.
//!
//! Client extension data (core §8, §8.1): an `extensions` field is an object whose members are
//! extension namespaces and whose values are objects. The kit checks that shape only: a namespace's
//! contents belong to the client that owns it, and the members of a namespace the host does not
//! implement are ignored without validating their values.

use indexmap::IndexMap;

use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;

/// The data of the namespaces a host implements, as written; namespaces it does not implement are
/// dropped.
pub type CoreExtensions = IndexMap<String, JsObject>;

/// Why an `extensions` value breaks core §8.1, or `None` when its shape is right.
#[must_use]
pub fn extensions_problem(value: &JsValue) -> Option<String> {
    let JsValue::Object(members) = value else {
        return Some("must be an object keyed by extension namespace".to_owned());
    };
    property_order(members)
        .into_iter()
        .find(|(_, data)| !is_record(data))
        .map(|(namespace, _)| format!("namespace \"{namespace}\" must hold an object"))
}

/// The members of a valid `extensions` value that the host implements (core §8.1: the others are
/// ignored). `None` when nothing implemented is there, so a result carries no empty object.
#[must_use]
pub fn implemented_extensions(value: &JsValue, implemented: &[String]) -> Option<CoreExtensions> {
    let JsValue::Object(members) = value else {
        return None;
    };
    let kept: CoreExtensions = property_order(members)
        .into_iter()
        .filter(|(namespace, _)| implemented.contains(namespace))
        .filter_map(|(namespace, data)| match data {
            JsValue::Object(record) => Some((namespace.clone(), record.clone())),
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::String(_)
            | JsValue::Array(_) => None,
        })
        .collect();
    if kept.is_empty() { None } else { Some(kept) }
}

crate::registry::exports! {
    "format/core/extensions" {
        "extensionsProblem" => ok |value: JsValue| extensions_problem(&value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> JsValue {
        crate::js::json_parse::json_parse(text).unwrap_or_default()
    }

    // bun -e 'import { extensionsProblem } from "./src/format/core/extensions.ts";
    //   console.log(extensionsProblem(JSON.parse(`{"z":1,"5":2,"a":{}}`)))'
    // prints: namespace "5" must hold an object (not "z", though it is written first)
    #[test]
    fn the_first_bad_namespace_is_the_first_object_key() {
        let problem = extensions_problem(&parsed(r#"{"z":1,"5":2,"a":{}}"#));
        assert_eq!(
            problem.as_deref(),
            Some("namespace \"5\" must hold an object")
        );
    }

    // bun -e 'import { implementedExtensions } from "./src/format/core/extensions.ts";
    //   console.log(JSON.stringify(implementedExtensions(JSON.parse(`{"z":{"x":1},"10":{"y":2},
    //   "3":{"w":3},"q":{}}`), ["z","10","3"])))'
    // prints {"3":{"w":3},"10":{"y":2},"z":{"x":1}}
    #[test]
    fn implemented_namespaces_list_integer_like_names_first() {
        let kept = implemented_extensions(
            &parsed(r#"{"z":{"x":1},"10":{"y":2},"3":{"w":3},"q":{}}"#),
            &["z".to_owned(), "10".to_owned(), "3".to_owned()],
        );
        let names: Vec<String> = kept
            .map(|kept| kept.into_keys().collect())
            .unwrap_or_default();
        assert_eq!(names, ["3", "10", "z"]);
    }
}
