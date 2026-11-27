//! Port of `src/hooks/input/permission-mode.ts`, switched.
//!
//! The `permission_mode` a hook receives (hooks.md, "Common input fields"): one of `"default"`,
//! `"plan"`, `"acceptEdits"`, `"auto"`, `"dontAsk"` or `"bypassPermissions"`. "The mode labeled
//! **Manual** arrives as `"default"`, never as `"manual"`, so scripts that match `"default"` keep
//! working." A host that labels its mode Manual passes it as it is called; a text that names no
//! mode yields `None`.

use crate::protocol::permission_modes::normalize_permission_mode;

/// The mode a hook receives for `mode`.
#[must_use]
pub fn hook_permission_mode(mode: &str) -> Option<&'static str> {
    normalize_permission_mode(mode)
}

crate::registry::exports! {
    "hooks/input/permission-mode" {
        "hookPermissionMode" => ok |mode: String| hook_permission_mode(&mode);
    }
}
