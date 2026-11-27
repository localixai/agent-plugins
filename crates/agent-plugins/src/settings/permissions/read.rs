//! Ported from `src/settings/permissions/read.ts`; only the core uses it now, so it has no wrapper.
//!
//! Reading the `permissions` block of one settings source. Claude Code "repairs the `permissions`
//! ... blocks per field instead of dropping them whole" (managed-settings.md, "Keys that fail
//! closed"): in the policy tier a lock inside one reads as its restrictive value, an invalid
//! `defaultMode` reads as `default`, and while a `deny` or `ask` list cannot be read at all `allow`
//! and `additionalDirectories` are withheld. Any other source loses the value or the entry
//! (settings.md, "Fix a broken settings file").

use crate::format::issues::{PluginIssue, warning};
use crate::format::permission_rule_shape::{MALFORMED_RULE, invalid_rule_message, split_rule};
use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::protocol::permission_modes::normalize_permission_mode;
use crate::settings::repair::support::{RepairAt, Repaired, dropped, failed_closed, kept, where_};

type FieldRead = fn(&JsValue, &RepairAt, &str) -> Repaired;

fn field_at(at: &RepairAt, field: &str) -> String {
    where_(at, &format!(".{field}"))
}

/// A list of texts (rules, directories): an entry that is no text is dropped, a non-list is
/// unreadable.
fn text_list(raw: &JsValue, at: &RepairAt, field: &str) -> Repaired {
    let Some(items) = raw.as_array() else {
        return kept(
            JsValue::Undefined,
            vec![warning(
                &field_at(at, field),
                &format!("\"permissions.{field}\" must be a list; it is dropped"),
            )],
        );
    };
    let mut entries = Vec::new();
    let mut issues = Vec::new();
    for (index, entry) in items.iter().enumerate() {
        if entry.is_string() {
            entries.push(entry.clone());
        } else {
            issues.push(warning(
                &where_(at, &format!(".{field}[{index}]")),
                "Expected a text; the entry is skipped",
            ));
        }
    }
    kept(JsValue::Array(entries), issues)
}

/// Rules are syntax-checked before any source can grant or restrict tools.
fn rule_list(raw: &JsValue, at: &RepairAt, field: &str) -> Repaired {
    let Some(items) = raw.as_array() else {
        return text_list(raw, at, field);
    };
    let mut entries = Vec::new();
    let mut issues = Vec::new();
    for (index, entry) in items.iter().enumerate() {
        if let Some(rule) = entry.as_str().filter(|rule| split_rule(rule).is_some()) {
            entries.push(JsValue::String(rule.to_owned()));
            continue;
        }
        let message = entry.as_str().map_or_else(
            || "A permission rule must be a string; the rule is skipped".to_owned(),
            |rule| invalid_rule_message(rule, MALFORMED_RULE),
        );
        issues.push(warning(
            &where_(at, &format!(".{field}[{index}]")),
            &message,
        ));
    }
    kept(JsValue::Array(entries), issues)
}

/// The value of a lock key: the string `disable`. Unreadable in the policy tier it reads as
/// `disable`.
fn lock_value(raw: &JsValue, at: &RepairAt, label: &str) -> Repaired {
    if raw.as_str() == Some("disable") {
        return kept(JsValue::String("disable".to_owned()), Vec::new());
    }
    if at.closed {
        return kept(
            JsValue::String("disable".to_owned()),
            vec![warning(
                &field_at(at, label),
                &format!("\"{label}\" was present but invalid; it is treated as \"disable\""),
            )],
        );
    }
    kept(
        JsValue::Undefined,
        vec![warning(
            &field_at(at, label),
            &format!("\"{label}\" must be \"disable\"; the key is ignored"),
        )],
    )
}

/// The issues of a field's read, each naming the field as `permissions.<field>`.
fn in_block(mut read: Repaired, field: &str) -> Repaired {
    for issue in &mut read.issues {
        issue.message = issue
            .message
            .replace(&format!("\"{field}\""), &format!("\"permissions.{field}\""));
    }
    read
}

fn lock_field(raw: &JsValue, at: &RepairAt, field: &str) -> Repaired {
    in_block(lock_value(raw, at, field), field)
}

/// `"true"` or `"false"` as text reads as that Boolean in the policy tier, with a notice.
#[must_use]
pub fn flag_value(raw: &JsValue, at: &RepairAt, label: &str, restrictive: bool) -> Repaired {
    if raw.is_boolean() {
        return kept(raw.clone(), Vec::new());
    }
    if !at.closed {
        return kept(
            JsValue::Undefined,
            vec![warning(
                &field_at(at, label),
                &format!("\"{label}\" must be true or false; it is dropped"),
            )],
        );
    }
    if let Some(text @ ("true" | "false")) = raw.as_str() {
        return kept(
            JsValue::Bool(text == "true"),
            vec![warning(
                &field_at(at, label),
                &format!("\"{label}\" is the text \"{text}\": drop the quotes"),
            )],
        );
    }
    kept(
        JsValue::Bool(restrictive),
        vec![warning(
            &field_at(at, label),
            &format!("\"{label}\" was present but invalid; it is treated as {restrictive}"),
        )],
    )
}

fn block_flag(raw: &JsValue, at: &RepairAt, field: &str) -> Repaired {
    in_block(flag_value(raw, at, field, true), field)
}

/// `defaultMode`: a mode or the alias `manual` (read as `default`); invalid, the policy tier reads
/// `default`.
fn mode_field(raw: &JsValue, at: &RepairAt, field: &str) -> Repaired {
    if let Some(mode) = raw.as_str().and_then(normalize_permission_mode) {
        return kept(JsValue::String(mode.to_owned()), Vec::new());
    }
    if at.closed {
        return kept(
            JsValue::String("default".to_owned()),
            vec![warning(
                &field_at(at, field),
                &format!(
                    "\"permissions.{field}\" was present but invalid; it is treated as \"default\""
                ),
            )],
        );
    }
    kept(
        JsValue::Undefined,
        vec![warning(
            &field_at(at, field),
            &format!("\"permissions.{field}\" is not a mode; it is dropped"),
        )],
    )
}

const FIELDS: [(&str, FieldRead); 10] = [
    ("allow", rule_list),
    ("ask", rule_list),
    ("deny", rule_list),
    ("additionalDirectories", text_list),
    ("defaultMode", mode_field),
    ("disableBypassPermissionsMode", lock_field),
    ("disableAutoMode", lock_field),
    ("denyInsteadOfPrompting", block_flag),
    ("blockReadsOutsideWorkingDirectories", block_flag),
    ("blockWritesOutsideWorkingDirectories", block_flag),
];

/// The restrictions whose loss withholds the grants written beside them.
const RESTRICTIONS: [&str; 2] = ["deny", "ask"];
/// The grants withheld while a restriction cannot be read.
const GRANTS: [&str; 2] = ["allow", "additionalDirectories"];

/// One field of the block: its repaired value, or the value as written for a field the kit does
/// not know.
fn read_field(name: &str, raw: &JsValue, at: &RepairAt) -> Repaired {
    FIELDS.iter().find(|(known, _)| *known == name).map_or_else(
        || kept(raw.clone(), Vec::new()),
        |(_, read)| read(raw, at, name),
    )
}

fn withheld(fields: &mut JsObject, unreadable: &[String], at: &RepairAt) -> Vec<PluginIssue> {
    if !at.closed || unreadable.is_empty() {
        return Vec::new();
    }
    let names = unreadable
        .iter()
        .map(|name| format!("\"permissions.{name}\""))
        .collect::<Vec<_>>()
        .join(" and ");
    GRANTS
        .iter()
        .filter_map(|grant| {
            fields.shift_remove(*grant).filter(|value| !value.is_undefined())?;
            Some(warning(
                &field_at(at, grant),
                &format!(
                    "\"permissions.{grant}\" is withheld: {names} could not be read, so grants never apply without the restrictions written beside them"
                ),
            ))
        })
        .collect()
}

/// `permissions` of one source, repaired per field.
#[must_use]
pub fn permissions_block(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(members) = value.as_object() else {
        // "The block is also on while Claude Code can't read the `permissions` object in your
        // managed settings."
        return if at.closed {
            failed_closed(
                at,
                JsValue::Object(JsObject::from([(
                    "blockWritesOutsideWorkingDirectories".to_owned(),
                    JsValue::Bool(true),
                )])),
                "a block with \"blockWritesOutsideWorkingDirectories\" on",
            )
        } else {
            dropped(at, &format!("\"{}\" must be an object", at.key))
        };
    };
    let mut fields = JsObject::new();
    let mut issues = Vec::new();
    let mut unreadable = Vec::new();
    for (name, raw) in property_order(members) {
        if raw.is_null() {
            continue;
        }
        let read = read_field(name, raw, at);
        issues.extend(read.issues);
        if !read.value.is_undefined() {
            fields.insert(name.clone(), read.value);
        } else if RESTRICTIONS.contains(&name.as_str()) {
            unreadable.push(name.clone());
        }
    }
    issues.extend(withheld(&mut fields, &unreadable, at));
    kept(JsValue::Object(in_property_order(fields)), issues)
}

/// `disableAutoMode` at the top level of a file.
#[must_use]
pub fn disable_value(value: &JsValue, at: &RepairAt) -> Repaired {
    lock_value(value, at, &at.key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::layers::PolicyScope;

    #[test]
    fn an_unreadable_deny_withholds_the_grants_in_the_policy_tier() {
        let at = RepairAt {
            key: "permissions".to_owned(),
            scope: PolicyScope::Managed,
            closed: true,
        };
        let block = crate::js::json_parse::json_parse(r#"{"allow":["Read"],"deny":"x"}"#)
            .unwrap_or_default();
        let read = permissions_block(&block, &at);
        assert_eq!(read.value.to_json_text(), "{}");
        assert_eq!(read.issues.len(), 2);
    }
}
