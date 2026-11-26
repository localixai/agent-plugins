//! Ported from `src/hooks/input/background-tasks.ts`; only the core uses it now, so it has no wrapper.
//!
//! The `background_tasks` and `session_crons` of a `Stop` or `SubagentStop` input (hooks.md, "Stop
//! input"): free text is "capped at 1000 characters with an in-string `… [+N chars]` marker when
//! clipped", and the fields only some task types have are "present only for" those types.

use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::js::value::JsObject;
use crate::js::{JsValue, code_unit_length};

/// One background task of the input (`BackgroundTask`).
pub type BackgroundTask = JsObject;
/// One session cron of the input (`SessionCron`).
pub type SessionCron = JsObject;

const TEXT_CAP: usize = 1000;

/// The text clipped to {@link TEXT_CAP} characters (Unicode code points, so a surrogate pair is not
/// split) with `… [+N chars]` appended, `N` being the characters cut. A text within the cap stays as
/// it is (spec/judgments.md: the docs do not say whether the marker counts toward the cap).
fn cap_text(text: &str) -> String {
    if code_unit_length(text) <= TEXT_CAP {
        return text.to_owned();
    }
    let characters = text.chars().count();
    if characters <= TEXT_CAP {
        return text.to_owned();
    }
    let cut = characters - TEXT_CAP;
    let kept: String = text.chars().take(TEXT_CAP).collect();
    format!("{kept}… [+{cut} chars]")
}

/// The same for a member that is a string; any other value stays as it is.
fn capped(value: &JsValue) -> JsValue {
    match value {
        JsValue::String(text) => JsValue::String(cap_text(text)),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::Array(_)
        | JsValue::Object(_) => value.clone(),
    }
}

/// The fields each task type carries beyond `id`, `type`, `status` and `description`.
const TASK_FIELDS: [(&str, &[&str]); 5] = [
    ("shell", &["command"]),
    ("subagent", &["agent_type"]),
    ("monitor", &["server", "tool"]),
    ("MCP task", &["server", "tool"]),
    ("workflow", &["name"]),
];

const TYPED_FIELDS: [&str; 5] = ["command", "agent_type", "server", "tool", "name"];

fn normalize_task(
    task: &BackgroundTask,
    path: &str,
    issues: &mut Vec<PluginIssue>,
) -> BackgroundTask {
    let kind = task
        .get("type")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    let allowed: &[&str] = TASK_FIELDS
        .iter()
        .find(|(name, _)| *name == kind)
        .map_or(&[], |(_, fields)| *fields);
    let mut kept = task.clone();
    if let Some(description) = task.get("description") {
        kept.insert("description".to_owned(), capped(description));
    }
    if let Some(command) = task.get("command").filter(|value| !value.is_undefined()) {
        kept.insert("command".to_owned(), capped(command));
    }
    for field in TYPED_FIELDS {
        if task.get(field).is_none_or(JsValue::is_undefined) || allowed.contains(&field) {
            continue;
        }
        kept.shift_remove(field);
        issues.push(warning(
            &format!("{path}.{field}"),
            &format!("\"{field}\" is not sent for a \"{kind}\" task"),
        ));
    }
    kept
}

/// What the tasks come to, with the issues of the fields dropped.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NormalizedTasks {
    /// The tasks, each capped and stripped.
    pub tasks: Vec<BackgroundTask>,
    /// A warning for each field dropped.
    pub issues: Vec<PluginIssue>,
}

/// Each task capped and stripped of the fields its type does not have; unknown types keep none.
#[must_use]
pub fn normalize_background_tasks(tasks: &[BackgroundTask], path: &str) -> NormalizedTasks {
    let mut issues = Vec::new();
    let tasks = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| normalize_task(task, &format!("{path}[{index}]"), &mut issues))
        .collect();
    NormalizedTasks { tasks, issues }
}

/// Each cron with its prompt capped.
#[must_use]
pub fn normalize_session_crons(crons: &[SessionCron]) -> Vec<SessionCron> {
    crons
        .iter()
        .map(|cron| {
            let mut kept = cron.clone();
            let prompt = cron.get("prompt").map_or(JsValue::Undefined, capped);
            kept.insert("prompt".to_owned(), prompt);
            kept
        })
        .collect()
}
