//! Ported from `src/hooks/select/mcp-tools.ts`; only the core uses it now, so it has no wrapper.
//!
//! Monitor calls (hooks.md, "Match Monitor commands"). A `PreToolUse` or `PermissionRequest` hook
//! with a `Bash` matcher also runs on a shell command the Monitor tool starts, but there the hook
//! gets a Bash call and its `allow` and `updatedInput` have no effect. MCP tool names need no code
//! of their own: `mcp__<server>__<tool>` is a tool name like any other, matched by the grammar.

use crate::hooks::contracts::{MonitorTreatment, ToolCallView};
use crate::js::JsValue;
use crate::rules::hook_matcher::{MatcherMode, matches_matcher};
use crate::rules::parse::{RuleOrigin, parse_rule};
use crate::rules::types::{PermissionBehavior, RuleSource};

/// The events whose `Bash` matcher also covers Monitor commands.
const COVERS_MONITOR: [(&str, bool); 33] = [
    ("PreToolUse", true),
    ("PermissionRequest", true),
    ("PostToolUse", false),
    ("PostToolUseFailure", false),
    ("PermissionDenied", false),
    ("PostToolBatch", false),
    ("Notification", false),
    ("UserPromptSubmit", false),
    ("UserPromptExpansion", false),
    ("SessionStart", false),
    ("SessionEnd", false),
    ("Stop", false),
    ("StopFailure", false),
    ("SubagentStart", false),
    ("SubagentStop", false),
    ("PreCompact", false),
    ("PostCompact", false),
    ("PreModelSwitch", false),
    ("PostModelSwitch", false),
    ("Setup", false),
    ("TeammateIdle", false),
    ("TaskCreated", false),
    ("TaskCompleted", false),
    ("Elicitation", false),
    ("ElicitationResult", false),
    ("ConfigChange", false),
    ("WorktreeCreate", false),
    ("WorktreeRemove", false),
    ("InstructionsLoaded", false),
    ("CwdChanged", false),
    ("FileChanged", false),
    ("DirectoryAdded", false),
    ("MessageDisplay", false),
];

fn covers_monitor(event: &str) -> bool {
    COVERS_MONITOR
        .iter()
        .any(|(name, covers)| *name == event && *covers)
}

/// A WebSocket watch has a `ws` input and no `command`: there is no shell command to match as Bash.
fn runs_command(input: &JsValue) -> bool {
    input.as_object().is_some() && input.get("command").is_some_and(JsValue::is_string)
}

/// Whether the hook's `if` rule names `Bash`: then `allow` and `updatedInput` no longer apply.
fn if_names_bash(if_rule: Option<&str>) -> bool {
    let Some(if_rule) = if_rule else {
        return false;
    };
    let at = RuleOrigin {
        behavior: PermissionBehavior::Deny,
        source: RuleSource::Cli,
    };
    parse_rule(if_rule, &at)
        .rule
        .is_some_and(|rule| rule.value.tool_name == "Bash")
}

/// The hook's matcher and `if` rule, as `monitorTreatment` reads them.
#[derive(Clone, Debug, Default)]
pub struct MonitorHook {
    /// The matcher of the group the handler was declared in.
    pub matcher: Option<String>,
    /// The handler's `if`.
    pub if_rule: Option<String>,
}

/// How a Monitor call reaches a hook whose matcher is `matcher`, or `None` when it does not.
/// `direct` is whether the matcher already selected the call by its own tool name (`Monitor`).
/// Matchers such as `Bash|PowerShell`, `^Bash$` and `Bash.*` are Bash matchers: they also select a
/// Monitor command, and a WebSocket watch is not one.
#[must_use]
pub fn monitor_treatment(
    event: &str,
    call: Option<&ToolCallView>,
    hook: &MonitorHook,
    direct: bool,
) -> Option<MonitorTreatment> {
    let call = call.filter(|call| call.tool_name.as_str() == Some("Monitor"))?;
    if !covers_monitor(event) {
        return None;
    }
    if direct {
        return Some(MonitorTreatment {
            as_bash: false,
            effects_apply: !if_names_bash(hook.if_rule.as_deref()),
        });
    }
    let as_bash = matches_matcher(hook.matcher.as_deref(), Some("Bash"), MatcherMode::Standard);
    if !as_bash || !runs_command(&call.tool_input) {
        return None;
    }
    Some(MonitorTreatment {
        as_bash: true,
        effects_apply: false,
    })
}
