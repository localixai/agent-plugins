//! Ported from `src/settings/general/checks.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a settings file may hold under each general key, checked as Claude Code does ("Settings
//! Warning: only individual entries fail, such as a malformed permission rule or an unknown hook
//! event name. Claude Code skips those values and keeps the rest of the file in effect",
//! settings.md, "Fix a broken settings file"). A value of the wrong type is dropped with a warning
//! that names the key; a malformed entry of a list is skipped on its own. In the policy tier a key
//! that fails closed reads as its restrictive value instead (managed-settings.md, "Keys that fail
//! closed"). The types are the SDK's `Settings` (docs/ARCHITECTURE.md, E2): these guards only read
//! them at run time.

use crate::js::JsValue;
use crate::js::json_stringify::json_stringify;
use crate::protocol::effort::EFFORT_LEVELS;
use crate::protocol::is_record::is_record;
use crate::protocol::settings_keys::CROSS_SESSION_MODES;
use crate::settings::env::check_env::env;
use crate::settings::general::closed_fields::{attribution, worktree};
use crate::settings::general::command_lines::command_line;
use crate::settings::general::look_and_feel::{TEAMMATE_MODES, parse_theme};
use crate::settings::general::sandbox_check::sandbox;
use crate::settings::repair::support::{
    RepairAt, Repaired, kept, must_be, note, present_but_invalid, strip_invalid_now,
};

/// A check of one key's value (`Repair`).
pub type Repair = fn(&JsValue, &RepairAt) -> Repaired;

fn boolean(value: &JsValue, at: &RepairAt) -> Repaired {
    if matches!(value, JsValue::Bool(_)) {
        kept(value.clone(), Vec::new())
    } else {
        must_be(at, "true or false")
    }
}

fn text(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.as_str().is_some() {
        kept(value.clone(), Vec::new())
    } else {
        must_be(at, "a string")
    }
}

fn one_of(allowed: &[&str], value: &JsValue, at: &RepairAt) -> Repaired {
    if value.as_str().is_some_and(|one| allowed.contains(&one)) {
        return kept(value.clone(), Vec::new());
    }
    let listed: Vec<String> = allowed.iter().map(|one| format!("\"{one}\"")).collect();
    must_be(at, &format!("one of {}", listed.join(", ")))
}

fn object(value: &JsValue, at: &RepairAt) -> Repaired {
    if is_record(value) {
        kept(value.clone(), Vec::new())
    } else {
        must_be(at, "an object")
    }
}

fn as_text(entry: &JsValue) -> Option<JsValue> {
    entry.as_str().map(|_| entry.clone())
}

fn text_list(value: &JsValue, at: &RepairAt) -> Repaired {
    let JsValue::Array(list) = value else {
        return must_be(at, "a list of strings");
    };
    let read = strip_invalid_now(
        list,
        at,
        as_text,
        &format!("\"{}\" entries must be strings", at.key),
    );
    kept(JsValue::Array(read.entries), read.issues)
}

/// `availableModels`: unreadable in managed settings it is "an empty allowlist until fixed, so only
/// the Default model is available"; a non-string entry is stripped and the rest enforced
/// (managed-settings.md, "Keys that fail closed").
fn model_allowlist(value: &JsValue, at: &RepairAt) -> Repaired {
    if matches!(value, JsValue::Array(_)) || !at.closed {
        return text_list(value, at);
    }
    kept(
        JsValue::Array(Vec::new()),
        vec![present_but_invalid(
            at,
            "it is an empty allowlist: only the Default model is available",
            "",
        )],
    )
}

fn is_cross_session_mode(value: &JsValue) -> bool {
    value
        .as_str()
        .is_some_and(|mode| CROSS_SESSION_MODES.contains(&mode))
}

/// `crossSessionInbound`: an unrecognized value is reported with the text of errors.md. In managed
/// settings it reads as `refuse`; in a user, project, local or `--settings` file it holds inbound
/// messages "even when a source that takes precedence sets `accept`", which the layer records
/// (`PolicyLayer.invalid`, the `unreadable` floor of the key's exception).
fn cross_session_inbound(value: &JsValue, at: &RepairAt) -> Repaired {
    if is_cross_session_mode(value) {
        return kept(value.clone(), Vec::new());
    }
    let modes: Vec<String> = CROSS_SESSION_MODES
        .iter()
        .map(|mode| format!("\"{mode}\""))
        .collect();
    let received = json_stringify(value).unwrap_or_else(|| "undefined".to_owned());
    let prefix = format!(
        "\"{}\" must be one of {}; received {received}. This value was ignored; while it is present, ",
        at.key,
        modes.join(", ")
    );
    if at.closed {
        return kept(
            JsValue::from("refuse"),
            vec![note(
                at,
                "",
                &format!(
                    "{prefix}cross-session messages are turned away until an administrator fixes it."
                ),
            )],
        );
    }
    Repaired {
        value: JsValue::Undefined,
        invalid: Some(true),
        issues: vec![note(
            at,
            "",
            &format!(
                "{prefix}cross-session messages are held for your approval instead of being delivered. Set it to one of the values above."
            ),
        )],
    }
}

fn any_value(value: &JsValue, _at: &RepairAt) -> Repaired {
    kept(value.clone(), Vec::new())
}

fn teammate_mode(value: &JsValue, at: &RepairAt) -> Repaired {
    one_of(&TEAMMATE_MODES, value, at)
}

fn theme(value: &JsValue, at: &RepairAt) -> Repaired {
    if value
        .as_str()
        .is_some_and(|name| parse_theme(name).is_some())
    {
        kept(value.clone(), Vec::new())
    } else {
        must_be(at, "a preset or custom theme reference")
    }
}

fn max_effort_level(value: &JsValue, at: &RepairAt) -> Repaired {
    one_of(EFFORT_LEVELS, value, at)
}

/// One check per general key. A dotted key lives inside a block and is read by its row's own
/// guard (`rows.ts`), so a file's top-level name of that shape is kept as written.
pub const GENERAL_CHECKS: [(&str, Repair); 34] = [
    ("agent", text),
    ("disableAgentView", boolean),
    ("outputStyle", text),
    ("processWrapper", text),
    ("statusLine", command_line),
    ("subagentStatusLine", command_line),
    ("fileSuggestion", command_line),
    ("teammateMode", teammate_mode),
    ("theme", theme),
    ("worktree.location", any_value),
    ("attribution", attribution),
    ("autoMode.classifyAllShell", any_value),
    ("availableModels", model_allowlist),
    ("crossSessionInbound", cross_session_inbound),
    ("enableArtifact", boolean),
    ("env", env),
    ("fallbackModel", text_list),
    ("isolatePeerMachines", boolean),
    ("maxEffortLevel", max_effort_level),
    ("model", text),
    ("modelPicker", object),
    ("permissions.blockReadsOutsideWorkingDirectories", any_value),
    (
        "permissions.blockWritesOutsideWorkingDirectories",
        any_value,
    ),
    ("permissions.defaultMode", any_value),
    ("permissions.denyInsteadOfPrompting", any_value),
    ("remoteControlAtStartup", boolean),
    ("sandbox", sandbox),
    ("sandbox.filesystem.allowManagedReadPathsOnly", any_value),
    ("sandbox.filesystem.allowRead", any_value),
    ("sandbox.filesystem.disabled", any_value),
    ("sandbox.network.allowManagedDomainsOnly", any_value),
    ("sandbox.network.allowedDomains", any_value),
    ("spinnerTipsEnabled", boolean),
    ("worktree", worktree),
];

/// `GENERAL_CHECKS[key]`.
#[must_use]
pub fn general_check(key: &str) -> Option<Repair> {
    GENERAL_CHECKS
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, check)| *check)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::general::keys::GENERAL_KEYS;
    use crate::settings::layers::PolicyScope;

    #[test]
    fn every_general_key_has_its_check() {
        assert!(GENERAL_KEYS.iter().all(|key| general_check(key).is_some()));
    }

    #[test]
    fn an_unknown_cross_session_mode_is_held_outside_the_policy_tier() {
        let at = RepairAt {
            key: "crossSessionInbound".to_owned(),
            scope: PolicyScope::User,
            closed: false,
        };
        let repaired = cross_session_inbound(&JsValue::from("open"), &at);
        assert_eq!(repaired.invalid, Some(true));
        assert!(
            repaired
                .issues
                .first()
                .is_some_and(|issue| issue.message.contains("received \"open\""))
        );
    }
}
