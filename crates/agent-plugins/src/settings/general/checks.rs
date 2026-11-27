//! Port of `src/settings/general/checks.ts`, switched.
//!
//! What a settings file may hold under each general key, checked as Claude Code does ("Settings
//! Warning: only individual entries fail, such as a malformed permission rule or an unknown hook
//! event name. Claude Code skips those values and keeps the rest of the file in effect", settings.md,
//! "Fix a broken settings file"). A value of the wrong type is dropped with a warning that names
//! the key; a malformed entry of a list is skipped on its own. In the policy tier a key that fails
//! closed reads as its restrictive value instead (managed-settings.md, "Keys that fail closed"). The
//! types are the SDK's `Settings` (docs/ARCHITECTURE.md, E2): these guards only read them at run time.

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

/// A key's check: the value as read, and where it was read.
pub type Repair = fn(&JsValue, &RepairAt) -> Repaired;

fn boolean(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_boolean() {
        kept(value.clone(), Vec::new())
    } else {
        must_be(at, "true or false")
    }
}

fn text(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_string() {
        kept(value.clone(), Vec::new())
    } else {
        must_be(at, "a string")
    }
}

fn one_of(value: &JsValue, at: &RepairAt, allowed: &[&str]) -> Repaired {
    if value.as_str().is_some_and(|value| allowed.contains(&value)) {
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
    entry.is_string().then(|| entry.clone())
}

fn text_list(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(list) = value.as_array() else {
        return must_be(at, "a list of strings");
    };
    let reason = format!("\"{}\" entries must be strings", at.key);
    let read = strip_invalid_now(list, at, as_text, &reason);
    kept(JsValue::Array(read.entries), read.issues)
}

/// `availableModels`: unreadable in managed settings it is "an empty allowlist until fixed, so only
/// the Default model is available"; a non-string entry is stripped and the rest enforced
/// (managed-settings.md, "Keys that fail closed").
fn model_allowlist(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_array() || !at.closed {
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
    CROSS_SESSION_MODES
        .iter()
        .any(|mode| value.as_str() == Some(*mode))
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
        let message = format!(
            "{prefix}cross-session messages are turned away until an administrator fixes it."
        );
        return kept(JsValue::from("refuse"), vec![note(at, "", &message)]);
    }
    let message = format!(
        "{prefix}cross-session messages are held for your approval instead of being delivered. Set it to one of the values above."
    );
    Repaired {
        value: JsValue::Undefined,
        invalid: Some(true),
        issues: vec![note(at, "", &message)],
    }
}

fn any_value(value: &JsValue, _at: &RepairAt) -> Repaired {
    kept(value.clone(), Vec::new())
}

fn theme(value: &JsValue, at: &RepairAt) -> Repaired {
    if value
        .as_str()
        .is_some_and(|value| parse_theme(value).is_some())
    {
        kept(value.clone(), Vec::new())
    } else {
        must_be(at, "a preset or custom theme reference")
    }
}

/// One check per general key. A dotted key lives inside a block and is read by its row's own guard
/// (`rows.ts`), so a file's top-level name of that shape is kept as written.
pub const GENERAL_CHECKS: [(&str, Repair); 34] = [
    ("agent", text),
    ("disableAgentView", boolean),
    ("outputStyle", text),
    ("processWrapper", text),
    ("statusLine", command_line),
    ("subagentStatusLine", command_line),
    ("fileSuggestion", command_line),
    ("teammateMode", |value, at| {
        one_of(value, at, &TEAMMATE_MODES)
    }),
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
    ("maxEffortLevel", |value, at| {
        one_of(value, at, EFFORT_LEVELS)
    }),
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

/// The check of a general key run on `value`, or why there is none: the key is no general key.
///
/// # Errors
///
/// When `key` has no row in `GENERAL_CHECKS`.
pub fn general_check(key: &str, value: &JsValue, at: &RepairAt) -> Result<Repaired, String> {
    GENERAL_CHECKS
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, check)| check(value, at))
        .ok_or_else(|| format!("\"{key}\" is no general settings key"))
}

crate::registry::exports! {
    "settings/general/checks" {
        "generalCheck" => fallible |key: String, value: JsValue, at: RepairAt|
            general_check(&key, &value, &at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::general::keys::GENERAL_KEYS;

    #[test]
    fn every_general_key_has_its_check() {
        for key in GENERAL_KEYS {
            assert!(GENERAL_CHECKS.iter().any(|(name, _)| *name == key), "{key}");
        }
    }
}
