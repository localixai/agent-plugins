//! Ported from `src/hooks/output/merge-simple.ts`; only the core uses it now, so it has no wrapper.
//!
//! The merges of the events whose output only adds fields: the session and prompt events, the
//! watch lists, the display text, `retry`, the worktree path and compaction instructions
//! (hooks.md, each event's "decision control" or "output" section).

use super::classifier_note::classifier_note;
use super::fields::{bool_in, specific_of, string_in, strings_in, text_in};
use super::merge_rules::{MergedRule, PlainTextAs, Read, last_of, nothing};
use super::tool_output::{check_tool_output, is_mcp_tool_name};
use super::worktree_path::last_path_line;
use crate::format::issues::{PluginIssue, warning};
use crate::hooks::input::fact_read::fact_of;
use crate::hooks::verdict::types::{MergedBlock, MergedVia};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, trim};

fn any_true(reads: &[&Read], key: &str) -> Option<bool> {
    reads
        .iter()
        .any(|read| bool_in(read.specific.as_ref(), key) == Some(true))
        .then_some(true)
}

fn optional(value: Option<String>) -> JsValue {
    value.map(JsValue::from).unwrap_or_default()
}

fn last_text(reads: &[Read], key: &str) -> Option<String> {
    let texts: Vec<JsValue> = reads
        .iter()
        .map(|read| optional(text_in(read.specific.as_ref(), key)))
        .collect();
    last_of(&texts).and_then(|text| text.as_str().map(str::to_owned))
}

/// Every hook's list, once each, in order: judgment (spec/judgments.md). `None` when no hook
/// returned one.
fn union_of_lists(reads: &[Read]) -> Option<Vec<String>> {
    let found: Vec<Option<Vec<String>>> = reads
        .iter()
        .map(|read| strings_in(read.specific.as_ref(), "watchPaths"))
        .collect();
    if found.iter().all(Option::is_none) {
        return None;
    }
    let mut union: Vec<String> = Vec::new();
    for path in found.into_iter().flatten().flatten() {
        if !union.contains(&path) {
            union.push(path);
        }
    }
    Some(union)
}

fn effects_of(fields: Vec<(&str, Option<JsValue>)>) -> JsObject {
    fields
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value)))
        .collect()
}

fn only(effects: JsObject) -> MergedRule {
    MergedRule {
        effects,
        issues: vec![],
        block: None,
    }
}

/// "Applies when `source` is `"startup"`, `"resume"`, or `"fork"`."
const TITLE_SOURCES: [&str; 3] = ["startup", "resume", "fork"];

/// `SessionStart`.
#[must_use]
pub fn merge_session_start(reads: &[Read], facts: &JsValue) -> MergedRule {
    let initial_user_message = last_text(reads, "initialUserMessage");
    let titled = fact_of(facts, "source")
        .as_str()
        .is_some_and(|source| TITLE_SOURCES.contains(&source));
    let session_title = if titled {
        last_text(reads, "sessionTitle")
    } else {
        None
    };
    let all: Vec<&Read> = reads.iter().collect();
    let reload_skills = any_true(&all, "reloadSkills");
    let watch_paths = union_of_lists(reads);
    only(effects_of(vec![
        (
            "initialUserMessage",
            initial_user_message.map(JsValue::from),
        ),
        ("sessionTitle", session_title.map(JsValue::from)),
        ("reloadSkills", reload_skills.map(JsValue::Bool)),
        ("watchPaths", watch_paths.map(strings)),
    ]))
}

fn strings(list: Vec<String>) -> JsValue {
    JsValue::Array(list.into_iter().map(JsValue::from).collect())
}

/// `suppressOriginalPrompt` is "if `true` when the hook blocks the prompt" (hooks.md,
/// "UserPromptSubmit decision control"), "whether the hook blocks with `decision: "block"` or by
/// exiting 2": only a hook that blocks by itself counts, so a hook that lets the prompt through
/// never changes another's block message (Agent Plugins Extended §7.5.8, Judgment J7.144).
#[must_use]
pub fn merge_user_prompt(reads: &[Read], _facts: &JsValue) -> MergedRule {
    let session_title = last_text(reads, "sessionTitle");
    let blocking: Vec<&Read> = reads.iter().filter(|read| read.blocks).collect();
    let suppress_original_prompt = any_true(&blocking, "suppressOriginalPrompt");
    only(effects_of(vec![
        ("sessionTitle", session_title.map(JsValue::from)),
        (
            "suppressOriginalPrompt",
            suppress_original_prompt.map(JsValue::Bool),
        ),
    ]))
}

/// `CwdChanged` and `FileChanged`.
#[must_use]
pub fn merge_watch_list(reads: &[Read], _facts: &JsValue) -> MergedRule {
    union_of_lists(reads).map_or_else(
        || nothing().into(),
        |watch_paths| only(effects_of(vec![("watchPaths", Some(strings(watch_paths)))])),
    )
}

/// `MessageDisplay`.
#[must_use]
pub fn merge_display(reads: &[Read], _facts: &JsValue) -> MergedRule {
    let texts: Vec<JsValue> = reads
        .iter()
        .map(|read| optional(string_in(read.specific.as_ref(), "displayContent")))
        .collect();
    last_of(&texts).map_or_else(
        || nothing().into(),
        |display_content| only(effects_of(vec![("displayContent", Some(display_content))])),
    )
}

/// "Claude Code ignores `retry: true` when the classifier produced no verdict on the action": the
/// denial `reason` then "starts with `Auto mode could not evaluate this action and is blocking it
/// for safety`", and "Claude Code already tells the model in the rejection message whether to
/// retry later or move on".
const NO_VERDICT_REASON: &str =
    "Auto mode could not evaluate this action and is blocking it for safety";

/// `PermissionDenied`.
#[must_use]
pub fn merge_retry(reads: &[Read], facts: &JsValue) -> MergedRule {
    let all: Vec<&Read> = reads.iter().collect();
    let Some(retry) = any_true(&all, "retry") else {
        return nothing().into();
    };
    let reason = fact_of(facts, "reason");
    if reason
        .as_str()
        .is_some_and(|reason| reason.starts_with(NO_VERDICT_REASON))
    {
        return MergedRule {
            effects: JsObject::new(),
            issues: vec![warning(
                "hooks.PermissionDenied.retry",
                "is ignored: the classifier produced no verdict on the action",
            )],
            block: None,
        };
    }
    only(effects_of(vec![("retry", Some(JsValue::Bool(retry)))]))
}

/// One hook's answer: an HTTP hook's `worktreePath`, or a command hook's last non-empty line.
fn worktree_answer_of(read: &Read) -> Option<String> {
    let given = string_in(read.specific.as_ref(), "worktreePath");
    if let Some(given) = given.as_deref().map(trim).filter(|given| !given.is_empty()) {
        return Some(given.to_owned());
    }
    read.stdout.as_deref().and_then(last_path_line)
}

/// Of several `WorktreeCreate` hooks the last answer in handler order is the path, an HTTP hook's
/// `worktreePath` and a command hook's stdout counting alike (Agent Plugins Extended §7.5.29,
/// Judgment J7.143).
#[must_use]
pub fn merge_worktree_create(reads: &[Read], _facts: &JsValue) -> MergedRule {
    let answers: Vec<JsValue> = reads
        .iter()
        .map(|read| optional(worktree_answer_of(read)))
        .collect();
    last_of(&answers).map_or_else(
        || nothing().into(),
        |worktree_path| only(effects_of(vec![("worktreePath", Some(worktree_path))])),
    )
}

/// "When a PreCompact hook exits 0 and doesn't block, Claude Code uses its stdout as
/// instructions."
#[must_use]
pub fn merge_pre_compact(reads: &[Read], _facts: &JsValue) -> MergedRule {
    let compaction_instructions: Vec<String> = reads
        .iter()
        .filter_map(|read| {
            read.plain_text
                .as_ref()
                .filter(|plain| plain.kind == PlainTextAs::CompactionInstructions)
                .map(|plain| plain.text.clone())
        })
        .collect();
    if compaction_instructions.is_empty() {
        return nothing().into();
    }
    only(effects_of(vec![(
        "compactionInstructions",
        Some(strings(compaction_instructions)),
    )]))
}

/// `Stop` and `SubagentStop` (hooks.md, "Stop decision control"): `additionalContext` is
/// "non-error feedback for Claude. The conversation continues so Claude can act on it". Judgment
/// (spec/judgments.md): a hook that also returns `continue: false` stops instead, since that
/// "takes precedence over any event-specific decision fields" (hooks.md, "JSON output").
#[must_use]
pub fn merge_stop(reads: &[Read], _facts: &JsValue) -> MergedRule {
    let feedback = reads.iter().any(|read| {
        !read.stops() && text_in(read.specific.as_ref(), "additionalContext").is_some()
    });
    if feedback {
        only(effects_of(vec![(
            "continuesTurn",
            Some(JsValue::Bool(true)),
        )]))
    } else {
        nothing().into()
    }
}

struct Rewrite {
    /// The index of the read it is of.
    read: usize,
    value: JsValue,
    accepted: bool,
}

/// `updatedToolOutput` of a hook, or for an MCP tool its `updatedMCPToolOutput` ("Replaces the
/// output for MCP tools only. Prefer `updatedToolOutput`"). A built-in tool's value that does not
/// match its output shape is "ignored and the original output is used"; an MCP tool's is passed
/// through.
fn rewrite_of(
    index: usize,
    read: &Read,
    specific: &JsObject,
    tool_name: &str,
) -> (Option<Rewrite>, Option<PluginIssue>) {
    let mcp = is_mcp_tool_name(tool_name);
    // A JSON `null` is a value: only `undefined` is "not returned".
    let defined = |key: &str| {
        specific
            .get(key)
            .filter(|value| !value.is_undefined())
            .cloned()
    };
    let own = defined("updatedToolOutput");
    let value = if own.is_none() && mcp {
        defined("updatedMCPToolOutput")
    } else {
        own
    };
    let Some(value) = value else {
        return (None, None);
    };
    let rewrite = |accepted: bool, value: JsValue| Rewrite {
        read: index,
        value,
        accepted,
    };
    if mcp {
        return (Some(rewrite(true, value)), None);
    }
    let checked = check_tool_output(tool_name, &value);
    if checked.accepted {
        return (Some(rewrite(true, value)), None);
    }
    let issue = warning(
        &format!("hooks.PostToolUse.{}.updatedToolOutput", read.handler.id),
        &format!(
            "does not match the {tool_name} output shape and is ignored ({})",
            checked.message.unwrap_or_default()
        ),
    );
    (Some(rewrite(false, value)), Some(issue))
}

/// `PostToolUse` (hooks.md, "PostToolUse decision control" and "Annotate a result for the auto
/// mode classifier"): the last accepted rewrite wins (spec/judgments.md); a classifier note is
/// dropped when its own rewrite is rejected or replaced by another hook's, and kept when it has no
/// rewrite.
#[must_use]
pub fn merge_post_tool_use(reads: &[Read], facts: &JsValue) -> MergedRule {
    let name = fact_of(facts, "tool_name")
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let (rewrites, issues): (Vec<Option<Rewrite>>, Vec<Option<PluginIssue>>) = reads
        .iter()
        .enumerate()
        .map(|(index, read)| {
            specific_of("PostToolUse", read.specific.as_ref())
                .and_then(JsValue::as_object)
                .map_or((None, None), |specific| {
                    rewrite_of(index, read, specific, &name)
                })
        })
        .unzip();
    let rewrites: Vec<Rewrite> = rewrites.into_iter().flatten().collect();
    let issues: Vec<PluginIssue> = issues.into_iter().flatten().collect();
    let winner = rewrites.iter().rfind(|rewrite| rewrite.accepted);
    // A note is dropped with its own rewrite when that is rejected or replaced, and kept otherwise.
    let kept: Vec<&Read> = reads
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            rewrites
                .iter()
                .find(|rewrite| rewrite.read == *index)
                .is_none_or(|own| own.accepted && winner.is_some_and(|won| won.read == own.read))
        })
        .map(|(_, read)| read)
        .collect();
    let note = classifier_note(&name, &kept);
    let mut effects = JsObject::new();
    if let Some(winner) = winner {
        effects.insert("updatedToolOutput".to_owned(), winner.value.clone());
    }
    if let Some(note) = note {
        effects.insert("classifierContext".to_owned(), JsValue::from(note.text));
        effects.insert(
            "classifierTrust".to_owned(),
            to_js_value(&note.trust).unwrap_or_default(),
        );
    }
    MergedRule {
        effects,
        issues,
        block: None,
    }
}

/// The answers that override every other.
const DECLINING: [&str; 1] = ["decline"];

struct Answer {
    action: String,
    content: Option<JsValue>,
}

/// A hook's answer: `decision: "block"` "overrides an `action` in the same output" and declines.
fn answer_of(read: &Read, event: &str) -> Option<Answer> {
    if read.decides_block() {
        return Some(Answer {
            action: "decline".to_owned(),
            content: None,
        });
    }
    let specific = specific_of(event, read.specific.as_ref())?.as_object()?;
    // "Claude Code ignores a `hookSpecificOutput` that has no `action`."
    let action = specific.get("action").and_then(JsValue::as_str)?;
    Some(Answer {
        action: action.to_owned(),
        content: specific
            .get("content")
            .filter(|content| !content.is_undefined())
            .cloned(),
    })
}

/// `Elicitation` and `ElicitationResult` (hooks.md, "Elicitation output", "ElicitationResult
/// output"): "a decline from one of them overrides an `accept` or `cancel` from another".
/// Judgment (spec/judgments.md): between an `accept` and a `cancel` the first hook in registry
/// order wins.
#[must_use]
pub fn merge_elicitation(event: &str, reads: &[Read]) -> MergedRule {
    let answers: Vec<Answer> = reads
        .iter()
        .filter_map(|read| answer_of(read, event))
        .collect();
    let Some(chosen) = answers
        .iter()
        .find(|answer| DECLINING.contains(&answer.action.as_str()))
        .or_else(|| answers.first())
    else {
        return nothing().into();
    };
    let blocking = reads.iter().find(|read| read.decides_block());
    let mut elicitation =
        JsObject::from_iter([("action".to_owned(), JsValue::from(chosen.action.as_str()))]);
    if let Some(content) = &chosen.content {
        elicitation.insert("content".to_owned(), content.clone());
    }
    MergedRule {
        effects: effects_of(vec![("elicitation", Some(JsValue::Object(elicitation)))]),
        issues: vec![],
        block: blocking.map(|read| MergedBlock {
            via: MergedVia::Decision,
            by: read.handler.id.clone(),
            reason: None,
        }),
    }
}

/// `TaskCompleted`'s `continue: false` holds only when a teammate's finished turn fired the
/// event: the input does not say which, so the merge marks the stop and the host applies the
/// condition.
#[must_use]
pub fn merge_task_completed(reads: &[Read], _facts: &JsValue) -> MergedRule {
    if reads.iter().any(Read::stops) {
        only(effects_of(vec![(
            "stopCondition",
            Some(JsValue::from("teammate-finished-its-turn")),
        )]))
    } else {
        nothing().into()
    }
}
