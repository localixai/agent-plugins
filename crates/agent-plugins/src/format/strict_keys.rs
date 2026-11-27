//! Ported from `src/format/strict-keys.ts`; only the core uses it now, so it has no wrapper.
//!
//! A strict object as Claude Code's schema check treats one (plugins/manifest-reference.md,
//! "Unrecognized fields": "An unknown key inside one is an error"): an unknown key is reported as
//! `Unrecognized key: "<key>"` (`Unrecognized keys: "a", "b"`) at the object, beside the problems of
//! the known fields rather than instead of them, so inside a union the object's own path is kept
//! (oracle 2.1.325: `lspServers.go: Unrecognized key: "foo"`, not a bare `Invalid input` at
//! `lspServers`). Zod's `strictObject` aborts on an unknown key, which a union then hides.

use crate::js::{JsValue, property_order};
use crate::zod::{self, Schema, Shape, issue};

/// `closedObject(shape)`: a loose object whose unknown keys are one `unrecognized_keys` issue that
/// lets the other checks go on.
#[must_use]
pub fn closed_object(shape: Shape) -> Schema {
    let known: Vec<String> = shape.keys().cloned().collect();
    zod::loose_object(shape).check(move |context| {
        let keys: Vec<JsValue> = context
            .value()
            .as_object()
            .map(|members| {
                property_order(members)
                    .into_iter()
                    .filter(|(key, _)| !known.contains(key))
                    .map(|(key, _)| JsValue::from(key.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        if keys.is_empty() {
            return;
        }
        let input = context.value().clone();
        context.push_issue(
            issue("unrecognized_keys")
                .set("keys", keys)
                .input(input)
                .continues(true),
        );
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    #[test]
    fn unknown_keys_are_one_issue_in_property_order() {
        let schema = closed_object(zod::fields! { "name" => zod::string() });
        let value = json_parse(r#"{"name": "x", "b": 0, "2": 0, "a": 0}"#).unwrap_or_default();
        let issues = schema.safe_parse(&value).err().map(|error| error.issues);
        let messages: Vec<String> = issues
            .unwrap_or_default()
            .iter()
            .map(|found| found.message().to_owned())
            .collect();
        assert_eq!(messages, [r#"Unrecognized keys: "2", "b", "a""#]);
    }
}
