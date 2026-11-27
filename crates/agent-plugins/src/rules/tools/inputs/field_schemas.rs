//! Port of `src/rules/tools/inputs/field-schemas.ts` at sha256:ec03868a2f22.
//!
//! The zod building blocks of the tool input tables (hooks.md, "PreToolUse input"). A message says
//! what is wrong with the value: `is required` for a field that is absent, `expected ...` for one of
//! another type. The issue path is added by `parseToolInput`.
//!
//! Every export is a schema or a function that builds one, so nothing here crosses the boundary:
//! the Rust modules that read tool inputs and responses build on these.

use std::sync::LazyLock;

use crate::zod::{self, Params, RawIssue, Schema, error_fn};

fn wrong_value(expected: &'static str) -> Params {
    error_fn(move |issue: &RawIssue| {
        if issue.input_value().is_undefined() {
            "is required".to_owned()
        } else {
            format!("expected {expected}")
        }
    })
}

/// `text`: a string.
pub static TEXT: LazyLock<Schema> = LazyLock::new(|| zod::string().error(wrong_value("a string")));
/// `num`: a number.
pub static NUM: LazyLock<Schema> = LazyLock::new(|| zod::number().error(wrong_value("a number")));
/// `flag`: a boolean.
pub static FLAG: LazyLock<Schema> =
    LazyLock::new(|| zod::boolean().error(wrong_value("a boolean")));

/// `list(item)`: an array of `item`.
#[must_use]
pub fn list(item: Schema) -> Schema {
    zod::array(item).error(wrong_value("an array"))
}

/// `record(value)`: an object of string keys whose every value is `value`.
#[must_use]
pub fn record(value: Schema) -> Schema {
    zod::record(zod::string(), value).error(wrong_value("an object"))
}

/// `choice(values)`: one of `values`.
#[must_use]
pub fn choice(values: &[String]) -> Schema {
    let joined = values.join(", ");
    zod::r#enum(values.iter().map(String::as_str)).error(error_fn(move |issue: &RawIssue| {
        if issue.input_value().is_undefined() {
            "is required".to_owned()
        } else {
            format!("expected one of {joined}")
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::JsValue;

    fn messages(schema: &Schema, value: &JsValue) -> Vec<String> {
        schema
            .safe_parse(value)
            .err()
            .map_or_else(Vec::new, |failure| {
                failure
                    .issues
                    .iter()
                    .map(|found| found.message().to_owned())
                    .collect()
            })
    }

    #[test]
    fn an_absent_value_is_required_and_a_wrong_one_names_what_was_expected() {
        assert_eq!(messages(&TEXT, &JsValue::Undefined), ["is required"]);
        assert_eq!(messages(&NUM, &JsValue::from("1")), ["expected a number"]);
        assert_eq!(
            messages(&list(TEXT.clone()), &JsValue::Null),
            ["expected an array"]
        );
        let modes = choice(&["a".to_owned(), "b".to_owned()]);
        assert_eq!(
            messages(&modes, &JsValue::from("c")),
            ["expected one of a, b"]
        );
        assert!(messages(&modes, &JsValue::from("b")).is_empty());
    }
}
