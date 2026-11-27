//! Port of `src/cli/flags/session.ts`, switched.
//!
//! `--permission-mode` and `--tools` (cli-reference.md, "CLI flags"). `--permission-mode` "accepts
//! `default`, `acceptEdits`, `plan`, `auto`, `dontAsk`, `bypassPermissions`, or `manual` as an alias
//! for `default`" and "overrides `defaultMode` from settings files"; a `--restricted` session "refuses
//! `bypassPermissions`". `--tools` "restricts which built-in tools Claude can use": `""` disables
//! all, `"default"` is the default set, or tool names like `"Bash,Edit,Read"`; naming a task-tracking
//! tool "also opts the session in". The wording of a refusal is in spec/judgments.md.

use super::types::{Contribution, FlagContext, refuse};
use crate::agents::permission_mode::permission_mode_inputs;
use crate::js::{JsValue, is_whitespace};

fn is_input(value: &str) -> bool {
    permission_mode_inputs().contains(&value)
}

/// `manual` is "an alias for `default`"; every other input is a mode.
fn mode_of(value: &str) -> &str {
    if value == "manual" { "default" } else { value }
}

/// `--permission-mode`.
#[must_use]
pub fn permission_mode(value: &str, context: &FlagContext) -> Contribution {
    if !is_input(value) {
        return refuse(vec![format!(
            "Error: Invalid permission mode: {value}. Use one of: {}",
            permission_mode_inputs().join(", ")
        )]);
    }
    let mode = mode_of(value);
    if mode == "bypassPermissions"
        && matches!(context.given.get("--restricted"), Some(JsValue::Bool(true)))
    {
        return refuse(vec![
            "Error: --restricted refuses --permission-mode bypassPermissions".to_owned(),
        ]);
    }
    Contribution {
        permission_mode: Some(mode.to_owned()),
        ..Contribution::default()
    }
}

/// The task-tracking tools: naming one in `--tools` opts the session in (tools-reference.md, "Task
/// tool availability").
const TASK_TOOLS: [&str; 5] = [
    "TaskCreate",
    "TaskGet",
    "TaskUpdate",
    "TaskList",
    "TodoWrite",
];

/// `/[\s,]+/`.
fn is_separator(character: char) -> bool {
    is_whitespace(character) || character == ','
}

/// `--tools`.
#[must_use]
pub fn tools(value: &str) -> Contribution {
    let names: Vec<String> = value
        .split(is_separator)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    let opts_into_task_tools = names.iter().any(|name| TASK_TOOLS.contains(&name.as_str()));
    Contribution {
        tools: Some(names),
        opts_into_task_tools: Some(opts_into_task_tools),
        ..Contribution::default()
    }
}

crate::registry::exports! {
    "cli/flags/session" {
        "permissionMode" => ok |value: String, context: FlagContext| permission_mode(&value, &context);
        "tools" => ok |value: String| tools(&value);
    }
}
