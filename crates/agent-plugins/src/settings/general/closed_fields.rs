//! Port of `src/settings/general/closed-fields.ts`, switched.
//!
//! The `worktree` and `attribution` blocks of a managed source, repaired per field instead of
//! dropped whole (managed-settings.md, "Keys that fail closed": "Claude Code repairs the
//! `permissions`, `autoMode`, `worktree`, and `attribution` blocks per field"). The docs give no
//! field rule for these two blocks: an invalid field is dropped with a warning and the field takes
//! its default, except `worktree.bgIsolation`, whose default `worktree` is the restrictive value
//! and which reads as it (spec/judgments.md). A field the kit does not know stays as written.

use crate::format::issues::PluginIssue;
use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::settings::repair::support::{
    RepairAt, Repaired, kept, must_be, note, present_but_invalid,
};

/// What a known field accepts, and what an invalid one reads as (`None`: it is dropped).
struct Field {
    valid: fn(&JsValue) -> bool,
    reads: Option<&'static str>,
}

fn texts(value: &JsValue) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(JsValue::is_string))
}

fn one_of(value: &JsValue, allowed: &[&str]) -> bool {
    value.as_str().is_some_and(|text| allowed.contains(&text))
}

/// `/^(?:\/|~\/|[A-Za-z]:[\\/])/`: an absolute path, one under the home folder, or a drive's.
fn is_location(value: &JsValue) -> bool {
    value.as_str().is_some_and(|text| {
        let mut characters = text.chars();
        let first = characters.next();
        let second = characters.next();
        let third = characters.next();
        first == Some('/')
            || (first == Some('~') && second == Some('/'))
            || (first.is_some_and(|letter| letter.is_ascii_alphabetic())
                && second == Some(':')
                && matches!(third, Some('\\' | '/')))
    })
}

fn worktree_field(name: &str) -> Option<Field> {
    let field = |valid, reads| Some(Field { valid, reads });
    match name {
        "symlinkDirectories" | "sparsePaths" => field(texts, None),
        "baseRef" => field(|value| one_of(value, &["fresh", "head"]), None),
        "bgIsolation" => field(
            |value| one_of(value, &["worktree", "none"]),
            Some("worktree"),
        ),
        "location" => field(is_location, None),
        _ => None,
    }
}

fn attribution_field(name: &str) -> Option<Field> {
    let field = |valid| Some(Field { valid, reads: None });
    match name {
        "commit" | "pr" => field(JsValue::is_string),
        "sessionUrl" => field(JsValue::is_boolean),
        _ => None,
    }
}

fn repair_field(at: &RepairAt, name: &str, reads: Option<&str>) -> (Option<JsValue>, PluginIssue) {
    if let Some(reads) = reads {
        return (
            Some(JsValue::String(reads.to_owned())),
            present_but_invalid(
                at,
                &format!("it is treated as \"{reads}\""),
                &format!(".{name}"),
            ),
        );
    }
    (
        None,
        note(
            at,
            &format!(".{name}"),
            &format!("\"{}.{name}\" is invalid; the field is dropped", at.key),
        ),
    )
}

/// The block with each invalid field repaired as `fields` says.
fn repair_fields(at: &RepairAt, value: &JsObject, fields: fn(&str) -> Option<Field>) -> Repaired {
    let mut issues = Vec::new();
    let mut entries = JsObject::new();
    for (name, one) in property_order(value) {
        let Some(field) = fields(name).filter(|field| !(field.valid)(one)) else {
            entries.insert(name.clone(), one.clone());
            continue;
        };
        let (repaired, issue) = repair_field(at, name, field.reads.filter(|_| at.closed));
        issues.push(issue);
        if let Some(repaired) = repaired {
            entries.insert(name.clone(), repaired);
        }
    }
    kept(JsValue::Object(in_property_order(entries)), issues)
}

fn block_check(
    value: &JsValue,
    at: &RepairAt,
    fields: fn(&str) -> Option<Field>,
    allows_boolean: bool,
) -> Repaired {
    if allows_boolean && value.is_boolean() {
        return kept(value.clone(), Vec::new());
    }
    let Some(members) = value.as_object() else {
        return must_be(
            at,
            if allows_boolean {
                "an object or a boolean"
            } else {
                "an object"
            },
        );
    };
    repair_fields(at, members, fields)
}

/// The repair of `worktree`.
#[must_use]
pub fn worktree(value: &JsValue, at: &RepairAt) -> Repaired {
    block_check(value, at, worktree_field, false)
}

/// `attribution` is also `false` ("to hide all attribution") or `true`.
#[must_use]
pub fn attribution(value: &JsValue, at: &RepairAt) -> Repaired {
    block_check(value, at, attribution_field, true)
}

crate::registry::exports! {
    "settings/general/closed-fields" {
        "worktree" => ok |value: JsValue, at: RepairAt| worktree(&value, &at);
        "attribution" => ok |value: JsValue, at: RepairAt| attribution(&value, &at);
    }
}

#[cfg(test)]
mod tests {
    use super::is_location;
    use crate::js::JsValue;

    #[test]
    fn a_location_is_absolute() {
        let text = |text: &str| JsValue::String(text.to_owned());
        assert!(is_location(&text("/x")));
        assert!(is_location(&text("~/x")));
        assert!(is_location(&text("C:\\x")));
        assert!(!is_location(&text("x")));
        assert!(!is_location(&text("~x")));
    }
}
