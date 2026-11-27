//! Port of `src/format/skill-field-readers.ts`, switched.
//!
//! Readers of the skill frontmatter fields (skills.md, "Frontmatter reference"). Each takes the
//! raw YAML value and its key and returns what it sets plus the problems with the value; a value
//! the field does not take is dropped with a problem, never thrown (the skill still loads).

use serde::Serialize;

use super::frontmatter_values::{FieldProblem, FieldRead, read_table, read_text};
use super::hooks_settings::{HooksReadOptions, read_hooks_settings};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, property_order, string_of, trim};

/// What one field sets: a piece of the skill, `name` (resolved elsewhere) included
/// (`SkillFieldValues`).
pub type SkillFieldValues = JsObject;

/// What one reader gives.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct FieldOutcome {
    /// What it sets.
    pub values: SkillFieldValues,
    /// What was wrong.
    pub problems: Vec<FieldProblem>,
}

/// Turn a `FieldRead` into an outcome: nothing is set for an absent or dropped value.
pub fn take<T>(read: FieldRead<T>, to_values: impl FnOnce(T) -> SkillFieldValues) -> FieldOutcome {
    FieldOutcome {
        values: read.value.map(to_values).unwrap_or_default(),
        problems: read.problem.into_iter().collect(),
    }
}

/// `{ <name>: value }`.
pub fn one<T: Serialize>(name: &str, value: &T) -> SkillFieldValues {
    JsObject::from([(name.to_owned(), to_js_value(value).unwrap_or_default())])
}

/// A YAML scalar written back as text; blank stays blank.
fn scalar_text(item: &JsValue) -> Option<String> {
    matches!(
        item,
        JsValue::String(_) | JsValue::Number(_) | JsValue::Bool(_)
    )
    .then(|| trim(&string_of(item)).to_owned())
}

/// One pair of a map item, as `key: value` (`[topic:]` is a null value, written `topic:`).
fn hint_pair(name: &str, value: &JsValue) -> Option<String> {
    if matches!(value, JsValue::Null) {
        return Some(format!("{name}:"));
    }
    scalar_text(value).map(|text| format!("{name}: {text}"))
}

/// A list item of an unquoted hint: a scalar, or a map such as `topic: foo | bar` written back.
fn hint_item(item: &JsValue) -> Option<String> {
    match item {
        JsValue::Object(members) => {
            let pairs: Option<Vec<String>> = property_order(members)
                .into_iter()
                .map(|(name, value)| hint_pair(name, value))
                .collect();
            pairs.map(|pairs| pairs.join(", "))
        }
        JsValue::Array(_) => None,
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_) => scalar_text(item),
    }
}

/// `argument-hint`, "[issue-number]" in the docs. Unquoted, that is a YAML flow list, so a list
/// is written back in its brackets, a map item as `key: value`: `[topic: foo | bar]`, the
/// changelog's example, reads as the text it was typed as (spec/judgments.md).
#[must_use]
pub fn read_argument_hint(value: &JsValue, key: &str) -> FieldOutcome {
    let JsValue::Array(list) = value else {
        return take(read_text(key, value), |hint| one("argumentHint", &hint));
    };
    let items: Vec<Option<String>> = list.iter().map(hint_item).collect();
    let parts: Vec<&str> = items
        .iter()
        .flatten()
        .map(String::as_str)
        .filter(|part| !part.is_empty())
        .collect();
    let mut problems = Vec::new();
    if items.iter().any(Option::is_none) {
        problems.push(FieldProblem {
            key: key.to_owned(),
            reason: "has an item that is not text".to_owned(),
        });
    }
    FieldOutcome {
        values: if parts.is_empty() {
            JsObject::new()
        } else {
            one("argumentHint", &format!("[{}]", parts.join(", ")))
        },
        problems,
    }
}

/// The Agent Skills spec's limit, quoted by skills.md: "a string of up to 500 characters".
pub const COMPATIBILITY_LIMIT: usize = 500;

/// `compatibility` is kept whole; a longer one is reported (spec/judgments.md: a warning).
#[must_use]
pub fn read_compatibility(value: &JsValue, key: &str) -> FieldOutcome {
    let read = read_text(key, value);
    let length = read.value.as_ref().map_or(0, |text| text.chars().count());
    let mut outcome = take(read, |compatibility| one("compatibility", &compatibility));
    if length > COMPATIBILITY_LIMIT {
        outcome.problems.push(FieldProblem {
            key: key.to_owned(),
            reason: format!("has {length} characters; the limit is {COMPATIBILITY_LIMIT}"),
        });
    }
    outcome
}

/// `hooks`, validated by the hooks reader and carried in the official shape. The skill still
/// loads when the reader refuses the block, only without its hooks (spec/judgments.md).
#[must_use]
pub fn read_hooks(value: &JsValue, key: &str) -> FieldOutcome {
    let table = read_table(key, value);
    let Some(members) = table.value else {
        return take(table, |_| JsObject::new());
    };
    let read = read_hooks_settings(&JsValue::Object(members), HooksReadOptions::default());
    let problems = read
        .issues
        .iter()
        .map(|found| FieldProblem {
            key: if found.path == key || found.path.starts_with(&format!("{key}.")) {
                found.path.clone()
            } else {
                format!("{key}.{}", found.path)
            },
            reason: found.message.clone(),
        })
        .collect();
    FieldOutcome {
        values: if read.hooks.is_empty() {
            JsObject::new()
        } else {
            one("hooks", &read.hooks)
        },
        problems,
    }
}

crate::registry::exports! {
    "format/skill-field-readers" {
        "COMPATIBILITY_LIMIT" => ok || COMPATIBILITY_LIMIT;
    }
}
