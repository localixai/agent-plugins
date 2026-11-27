//! Port of `src/rules/decide/hook-rewrite.ts`, switched.
//!
//! A PreToolUse hook that returns `updatedInput` with no `permissionDecision` in auto mode (hooks.md,
//! "PreToolUse `updatedInput` in auto mode"): "Claude Code can deny the changed call", because "the
//! server reviewed the input Claude wrote, not the input your hook returned". This says what becomes
//! of the change. The review itself, the denial and the next response are the host's.

use serde::{Deserialize, Serialize};

use super::context::{HookPermissionDecision, PermissionMode};
use crate::format::canonical_json::canonical_json;
use crate::js::JsValue;

/// "In interactive terminal sessions and VS Code sessions on a direct connection ... the denial includes this text".
pub const CHANGED_INPUT_DENIAL: &str =
    "a hook changed this call's input after auto mode reviewed it";

/// The calls a `command` hook may change without the denial (in the sessions that show the text).
const UNREVIEWED_CHANGES_OK: [&str; 6] = [
    "Bash",
    "PowerShell",
    "Monitor",
    "Edit",
    "Write",
    "NotebookEdit",
];

/// A call whose input a hook changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookRewriteCall {
    /// The tool called.
    pub tool_name: String,
    /// The input Claude wrote: what the server reviewed (on a repeat, the changed input Claude repeated).
    #[serde(default)]
    pub input: JsValue,
    /// What the hook returned.
    #[serde(default)]
    pub updated_input: JsValue,
    /// The hook's `permissionDecision`; the denial applies only to a change with none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<HookPermissionDecision>,
}

/// The session the change is made in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookRewriteSession {
    /// The session's permission mode.
    pub mode: PermissionMode,
    /// An interactive terminal or a VS Code session on a direct connection: the sessions that show the text.
    pub shows_denial_text: bool,
    /// The hook is a `command` hook in settings or in a plugin's hooks file.
    pub command_hook: bool,
    /// Any hook changed an earlier call in the same Claude response.
    pub earlier_change_in_response: bool,
    /// The changed input of a call that was denied for this reason, so a repeat of it can be told.
    #[serde(default)]
    pub denied_change: JsValue,
}

/// `apply`: the change runs. `deny`: auto mode refuses the changed call. `drop`: what runs is what the server reviewed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum HookRewriteVerdict {
    /// The change runs.
    Apply,
    /// What runs is what the server reviewed.
    Drop,
    /// Auto mode refuses the changed call.
    Deny {
        /// The text of the denial, in the sessions that show it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
}

const fn reviews(mode: PermissionMode) -> bool {
    match mode {
        PermissionMode::Auto => true,
        PermissionMode::Default
        | PermissionMode::AcceptEdits
        | PermissionMode::BypassPermissions
        | PermissionMode::Plan
        | PermissionMode::DontAsk => false,
    }
}

const APPLY: HookRewriteVerdict = HookRewriteVerdict::Apply;

/// A Bash command that "asks to run outside the sandbox".
fn asks_outside_sandbox(call: &HookRewriteCall) -> bool {
    call.tool_name == "Bash"
        && call.input.get("dangerouslyDisableSandbox") == Some(&JsValue::Bool(true))
}

/// Whether the session shows the denial and the call is one a command hook may change without it.
fn change_is_tolerated(call: &HookRewriteCall, session: &HookRewriteSession) -> bool {
    session.shows_denial_text
        && session.command_hook
        && UNREVIEWED_CHANGES_OK.contains(&call.tool_name.as_str())
        && !session.earlier_change_in_response
        && !asks_outside_sandbox(call)
}

/// What auto mode does with a hook's `updatedInput`. A call Claude repeats with exactly the changed
/// input that was denied is a repeat: "your hook still runs ... Any `updatedInput` it returns is
/// dropped". Elsewhere the change is denied unless the hook decided the call itself, or the session
/// and the call are among the ones that "change ... without this denial". The docs describe the
/// exemption only for the sessions that show the text, so elsewhere a change is denied, without the
/// text (spec/judgments.md).
#[must_use]
pub fn review_hook_rewrite(
    call: &HookRewriteCall,
    session: &HookRewriteSession,
) -> HookRewriteVerdict {
    if !reviews(session.mode) || call.updated_input.is_undefined() {
        return APPLY;
    }
    let repeated = !session.denied_change.is_undefined()
        && canonical_json(&call.input) == canonical_json(&session.denied_change);
    if repeated {
        return HookRewriteVerdict::Drop;
    }
    if call.decision.is_some() || change_is_tolerated(call, session) {
        return APPLY;
    }
    HookRewriteVerdict::Deny {
        text: session
            .shows_denial_text
            .then(|| CHANGED_INPUT_DENIAL.to_owned()),
    }
}

crate::registry::exports! {
    "rules/decide/hook-rewrite" {
        "CHANGED_INPUT_DENIAL" => ok || CHANGED_INPUT_DENIAL;
        "reviewHookRewrite" => ok |call: HookRewriteCall, session: HookRewriteSession| review_hook_rewrite(&call, &session);
    }
}
