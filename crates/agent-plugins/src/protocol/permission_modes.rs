//! Port of `src/protocol/permission-modes.ts`, switched.
//!
//! The permission modes of permissions.md ("Permission modes"), tied to the official
//! `PermissionMode` union: an SDK update that adds or drops a mode fails the typecheck of the
//! TypeScript wrapper.

/// The permission modes of permissions.md ("Permission modes"), tied to the official
/// `PermissionMode` union.
pub const PERMISSION_MODES: &[&str] = &[
    "default",
    "acceptEdits",
    "bypassPermissions",
    "plan",
    "dontAsk",
    "auto",
];

/// The three rule behaviours of permissions.md ("Manage permissions"): allow, ask and deny.
pub const PERMISSION_BEHAVIORS: &[&str] = &["allow", "deny", "ask"];

/// Other spellings of a mode: the `default` row of permissions.md ("Permission modes") says it is
/// "Labeled Manual in the CLI … and Claude Code accepts `manual` as an alias".
const PERMISSION_MODE_ALIASES: [(&str, &str); 1] = [("manual", "default")];

/// A mode name as Claude Code accepts it: a mode itself or one of its aliases; else `None`.
#[must_use]
pub fn normalize_permission_mode(raw: &str) -> Option<&'static str> {
    PERMISSION_MODES
        .iter()
        .find(|mode| **mode == raw)
        .copied()
        .or_else(|| {
            PERMISSION_MODE_ALIASES
                .iter()
                .find(|(alias, _)| *alias == raw)
                .map(|(_, mode)| *mode)
        })
}

crate::registry::exports! {
    "protocol/permission-modes" {
        "PERMISSION_MODES" => ok || PERMISSION_MODES;
        "PERMISSION_BEHAVIORS" => ok || PERMISSION_BEHAVIORS;
        "normalizePermissionMode" => ok |raw: String| normalize_permission_mode(&raw);
    }
}
