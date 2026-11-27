//! Ported from `src/hooks/verdict/notices.ts`; only the core uses it now, so it has no wrapper.
//!
//! The words the docs quote for a hook's errors (hooks.md, "Exit code 2", "Other exit codes",
//! "Block the action when a hook fails"), kept in one file so each is checked against its source.

use crate::js::value::JsObject;
use crate::js::{JsValue, number_to_string, trim};

/// `PreToolUse:Bash`: the event, and the tool when the event's input has one.
#[must_use]
pub fn hook_name(event: &str, tool_name: Option<&str>) -> String {
    match tool_name {
        None => event.to_owned(),
        Some(tool) => format!("{event}:{tool}"),
    }
}

/// A member as a template literal writes it.
fn text_of(member: Option<&JsValue>) -> String {
    match member {
        Some(JsValue::String(text)) => text.clone(),
        Some(JsValue::Number(number)) => number_to_string(*number),
        Some(JsValue::Bool(flag)) => flag.to_string(),
        Some(JsValue::Null) => "null".to_owned(),
        Some(JsValue::Undefined) | None => "undefined".to_owned(),
        Some(other @ (JsValue::Array(_) | JsValue::Object(_))) => other.to_json_text(),
    }
}

/// The part of a handler the docs put in brackets: a command hook's command and arguments ("[node
/// ${CLAUDE_PROJECT_DIR}/.claude/hooks/check-command.js]"). Judgment (spec/judgments.md): the other
/// types show their URL, `server:tool`, or their type.
#[must_use]
pub fn handler_label(handler: &JsObject) -> String {
    if handler.contains_key("command") {
        let mut words = vec![text_of(handler.get("command"))];
        if let Some(JsValue::Array(args)) = handler.get("args") {
            words.extend(args.iter().map(|arg| text_of(Some(arg))));
        }
        return words.join(" ");
    }
    if handler.contains_key("url") {
        return text_of(handler.get("url"));
    }
    if handler.contains_key("server") {
        return format!(
            "{}:{}",
            text_of(handler.get("server")),
            text_of(handler.get("tool"))
        );
    }
    text_of(handler.get("type"))
}

/// "PreToolUse:Bash hook error: [${CLAUDE_PROJECT_DIR}/.claude/hooks/no-rm.sh]: Blocked: rm commands are not allowed".
#[must_use]
pub fn hook_error(name: &str, label: &str, text: &str) -> String {
    format!("{name} hook error: [{label}]: {text}")
}

/// "Failed with non-blocking status code: something broke".
pub const NON_BLOCKING_STATUS: &str = "Failed with non-blocking status code:";

/// Judgment (spec/judgments.md): the docs never show the notice of a hook that wrote nothing to stderr.
pub const NO_STDERR: &str = "no stderr output";

/// "the error includes `failed; blocking because onFailure is "block"` followed by node's own error output".
#[must_use]
pub fn failed_blocking(detail: &str) -> String {
    with_detail("failed; blocking because onFailure is \"block\"", detail)
}

/// "After a timeout, the message says `timed out` instead of `failed`."
#[must_use]
pub fn timed_out_blocking(detail: &str) -> String {
    with_detail("timed out; blocking because onFailure is \"block\"", detail)
}

/// "If the error for a blocked tool call includes this text, a `PreToolUse` or `PermissionRequest`
/// command hook failed or timed out and blocked the call".
#[must_use]
pub fn windows_blocking(detail: &str) -> String {
    with_detail(
        "blocking because Git Bash wasn't found, so only exit code 0 or 2 counts as this hook's verdict.",
        detail,
    )
}

/// The word of a run that reached its limit.
pub const TIMED_OUT: &str = "timed out";

/// `PreModelSwitch`: a hook that doesn't respond before its timeout blocks the switch, with the
/// reason `PreModelSwitch hook <name> did not respond before its timeout` (errors.md, "Model switch
/// was blocked by a PreModelSwitch hook"; Agent Plugins Extended §7.5.33). The hook is named by its
/// handler label, as the hook error line names it.
#[must_use]
pub fn timed_out_blocks_switch(name: &str) -> String {
    format!("PreModelSwitch hook {name} did not respond before its timeout")
}

/// "The transcript shows a notice naming the hook, the timeout that fired, and that the output was
/// discarded" (`UserPromptSubmit`). Judgment (spec/judgments.md): the docs give the notice no
/// wording; the hook is named by the line around it, the timeout is written in seconds.
#[must_use]
pub fn timed_out_discarded(timeout_ms: Option<f64>) -> String {
    let after = timeout_ms.map_or_else(String::new, |ms| {
        format!(" after {}s", number_to_string(ms / 1000.0))
    });
    format!("{TIMED_OUT}{after}; the hook's output was discarded")
}

fn with_detail(head: &str, detail: &str) -> String {
    if trim(detail).is_empty() {
        head.to_owned()
    } else {
        format!("{head}\n{}", trim(detail))
    }
}

/// The first line of a text, trimmed.
#[must_use]
pub fn first_line(text: &str) -> String {
    let trimmed = trim(text);
    let line = trimmed.split_once('\n').map_or(trimmed, |(first, _)| first);
    line.strip_suffix('\r').unwrap_or(line).to_owned()
}

/// "When a plugin's hook blocks this way and its stderr is the blocking message, the error ends with
/// `This hook comes from the <plugin> plugin.` so that you know which plugin to disable or fix"
/// (plugins/troubleshooting.md). Judgment (spec/judgments.md): the docs give no separator, a space.
#[must_use]
pub fn from_plugin(message: &str, plugin: Option<&str>) -> String {
    match plugin {
        None => message.to_owned(),
        Some(plugin) => format!("{message} This hook comes from the {plugin} plugin."),
    }
}
