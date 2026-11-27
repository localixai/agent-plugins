//! Ported from `src/settings/env/check-env.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a file may hold under `env` (settings-reference.md, `env`: "object mapping variable names to
//! string values"; "Variables Claude Code ignores in `env`"): the variables Claude Code drops from the
//! file's scope are removed, named and never with their value, and so is an entry whose name no
//! process could carry. A value that is not a string is read as JavaScript's `String(value)`, as
//! claude 2.1.321 sets it (`1` is "1", `true` "true", `null` "null", `[1,2]` "1,2"), and is reported
//! so the author sees it (spec/judgments.md).

use super::ignored_variables::{IgnoreReason, env_text, ignored_because};
use crate::format::issues::PluginIssue;
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;
use crate::settings::broken_file::scope_label;
use crate::settings::layers::PolicyScope;
use crate::settings::repair::support::{RepairAt, Repaired, kept, must_be, note};

fn has_possible_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['=', '\0'])
}

const NO_FILE_REASONS: [IgnoreReason; 2] = [IgnoreReason::Identity, IgnoreReason::LaunchOnly];

fn problem(scope: PolicyScope, name: &str, value: &str) -> Option<String> {
    if !has_possible_name(name) {
        return Some(format!(
            "\"{name}\" cannot be the name of an environment variable"
        ));
    }
    let reason = ignored_because(scope, name, &JsValue::String(value.to_owned()), false)?;
    let who = if NO_FILE_REASONS.contains(&reason) {
        "no settings file can set it"
    } else {
        "a checked-out repository cannot set it"
    };
    Some(format!(
        "\"{name}\" is ignored from {}: {who}",
        scope_label(scope)
    ))
}

fn coerced(at: &RepairAt, name: &str, value: &JsValue) -> Vec<PluginIssue> {
    if value.is_string() {
        Vec::new()
    } else {
        vec![note(
            at,
            &format!(".{name}"),
            &format!(
                "\"{name}\" is not a string; it is read as \"{}\"",
                env_text(value)
            ),
        )]
    }
}

/// `env`: the entries Claude Code ignores or cannot read are removed, each reported by name.
#[must_use]
pub fn env(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(members) = value.as_object().filter(|_| is_record(value)) else {
        return must_be(at, "an object");
    };
    let mut issues = Vec::new();
    let mut entries = JsObject::new();
    for (name, one) in property_order(members) {
        let text = env_text(one);
        match problem(at.scope, name, &text) {
            Some(message) => issues.push(note(at, &format!(".{name}"), &message)),
            None => {
                issues.extend(coerced(at, name, one));
                entries.insert(name.clone(), JsValue::String(text));
            }
        }
    }
    kept(JsValue::Object(entries), issues)
}
