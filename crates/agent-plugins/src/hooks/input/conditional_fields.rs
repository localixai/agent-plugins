//! Ported from `src/hooks/input/conditional-fields.ts`; only the core uses it now, so it has no wrapper.
//!
//! Fields an event's input has only in some cases (hooks.md, the input tables). A host that supplies
//! one where the docs say it is absent gets it dropped, with a warning, so a hook never sees a field
//! Claude Code would not send.

use serde::Serialize;

use super::background_tasks::{
    BackgroundTask, SessionCron, normalize_background_tasks, normalize_session_crons,
};
use crate::format::issues::{PluginIssue, warning};
use crate::js::JsValue;
use crate::js::value::JsObject;

/// The facts of an event as its input has them, and what was found on the way.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Refined {
    /// The facts.
    pub facts: JsObject,
    /// A warning for each field dropped.
    pub issues: Vec<PluginIssue>,
}

fn unchanged(facts: &JsObject) -> Refined {
    Refined {
        facts: facts.clone(),
        issues: Vec::new(),
    }
}

/// `facts` without the named fields, and a warning for each one that was there.
fn drop_fields(event: &str, facts: &JsObject, fields: &[&str], because: &str) -> Refined {
    let present: Vec<&str> = fields
        .iter()
        .copied()
        .filter(|field| facts.contains_key(*field))
        .collect();
    if present.is_empty() {
        return unchanged(facts);
    }
    let mut kept = facts.clone();
    for field in &present {
        kept.shift_remove(*field);
    }
    let issues = present
        .iter()
        .map(|field| {
            warning(
                &format!("hooks.{event}.input.{field}"),
                &format!("\"{field}\" is only sent {because} and is dropped"),
            )
        })
        .collect();
    Refined {
        facts: kept,
        issues,
    }
}

/// `InstructionsLoaded`: `globs` "Present only for `path_glob_match` loads", `trigger_file_path`
/// "for lazy loads" (`nested_traversal`, `path_glob_match`), `parent_file_path` "for `include` loads".
fn load_fields(reason: &str) -> &'static [&'static str] {
    match reason {
        "nested_traversal" => &["trigger_file_path"],
        "path_glob_match" => &["globs", "trigger_file_path"],
        "include" => &["parent_file_path"],
        _ => &[],
    }
}
const ALL_LOAD_FIELDS: [&str; 3] = ["globs", "trigger_file_path", "parent_file_path"];

fn instructions_loaded(facts: &JsObject) -> Refined {
    let reason = facts
        .get("load_reason")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    let wanted = load_fields(reason);
    let unwanted: Vec<&str> = ALL_LOAD_FIELDS
        .iter()
        .copied()
        .filter(|field| !wanted.contains(field))
        .collect();
    drop_fields(
        "InstructionsLoaded",
        facts,
        &unwanted,
        &format!("for a matching load (not {reason})"),
    )
}

/// `SessionStart`: when `source` is `"resume"` or `"fork"` the hook also receives the four cost
/// fields; for any other source they are absent.
const COST_FIELDS: [&str; 4] = [
    "seconds_since_last_response",
    "context_tokens",
    "prompt_cache_likely_expired",
    "estimated_cache_write_usd",
];

fn has_cost_fields(source: &str) -> bool {
    matches!(source, "resume" | "fork")
}

fn session_start(facts: &JsObject) -> Refined {
    let source = facts
        .get("source")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    if has_cost_fields(source) {
        unchanged(facts)
    } else {
        drop_fields(
            "SessionStart",
            facts,
            &COST_FIELDS,
            "when the session is resumed or forked",
        )
    }
}

/// The objects of an array fact.
fn objects(value: &JsValue) -> Vec<JsObject> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_object().cloned())
                .collect()
        })
        .unwrap_or_default()
}

/// `Stop` and `SubagentStop`: capped free text and per-type task fields.
fn refine_stop(event: &str, facts: &JsObject) -> Refined {
    let mut issues = Vec::new();
    let mut refined = facts.clone();
    if let Some(tasks) = facts
        .get("background_tasks")
        .filter(|tasks| !matches!(tasks, JsValue::Undefined))
    {
        let given: Vec<BackgroundTask> = objects(tasks);
        let normalized =
            normalize_background_tasks(&given, &format!("hooks.{event}.input.background_tasks"));
        issues.extend(normalized.issues);
        refined.insert(
            "background_tasks".to_owned(),
            JsValue::Array(normalized.tasks.into_iter().map(JsValue::Object).collect()),
        );
    }
    if let Some(crons) = facts
        .get("session_crons")
        .filter(|crons| !matches!(crons, JsValue::Undefined))
    {
        let given: Vec<SessionCron> = objects(crons);
        refined.insert(
            "session_crons".to_owned(),
            JsValue::Array(
                normalize_session_crons(&given)
                    .into_iter()
                    .map(JsValue::Object)
                    .collect(),
            ),
        );
    }
    Refined {
        facts: refined,
        issues,
    }
}

/// `facts` as `event`'s input has them: the conditional fields kept or dropped as the docs say.
#[must_use]
pub fn apply_conditional_fields(event: &str, facts: &JsObject) -> Refined {
    match event {
        "InstructionsLoaded" => instructions_loaded(facts),
        "SessionStart" => session_start(facts),
        "Stop" | "SubagentStop" => refine_stop(event, facts),
        _ => unchanged(facts),
    }
}
