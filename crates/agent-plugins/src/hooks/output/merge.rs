//! Port of `src/hooks/output/merge.ts`, switched.
//!
//! The merge of every hook's output on one event (hooks.md, "JSON output", "Decision control"):
//! what each event honours (`OUTPUT_CAPS`), the universal fields, the context every hook adds, and
//! the event's own rule from `MERGERS`.

use serde::Deserialize;

use super::caps::output_caps;
use super::fields::text_in;
use super::merge_gates::{merge_permission_request, merge_pre_model_switch, merge_pre_tool_use};
use super::merge_rules::{MergedRule, PlainText, PlainTextAs, Read, ReadHandler, nothing};
use super::merge_simple::{
    merge_display, merge_elicitation, merge_post_tool_use, merge_pre_compact, merge_retry,
    merge_session_start, merge_stop, merge_task_completed, merge_user_prompt, merge_watch_list,
    merge_worktree_create,
};
use super::messages::{MessageRead, route_messages};
use super::reminder_tags::escape_reminder_tags;
use super::terminal_sequence::allowed_terminal_sequence;
use crate::format::issues::{PluginIssue, warning};
use crate::hooks::verdict::exit2::exit2_rule;
use crate::hooks::verdict::types::{MergedBlock, MergedOutput, MergedVia};
use crate::js::{JsObject, JsValue};

/// One hook's output as the verdict stage tagged it (`TaggedOutput`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggedOutput {
    /// The handler.
    pub handler: ReadHandler,
    /// The JSON fields that apply.
    #[serde(default)]
    pub output: Option<JsObject>,
    /// Plain stdout the event reads.
    #[serde(default)]
    pub plain_text: Option<PlainText>,
    /// A Monitor call reached the hook through a `Bash` matcher.
    #[serde(default)]
    pub effects_ignored: Option<bool>,
    /// The run held the action back by itself (exit 2, a failure that blocks), JSON aside.
    #[serde(default)]
    pub blocked: Option<bool>,
    /// Raw stdout of a `WorktreeCreate` command hook.
    #[serde(default)]
    pub stdout: Option<String>,
}

type Merger = fn(&[Read], &JsValue) -> MergedRule;

fn none(_reads: &[Read], _facts: &JsValue) -> MergedRule {
    nothing().into()
}

fn elicitation(reads: &[Read], _facts: &JsValue) -> MergedRule {
    merge_elicitation("Elicitation", reads)
}

fn elicitation_result(reads: &[Read], _facts: &JsValue) -> MergedRule {
    merge_elicitation("ElicitationResult", reads)
}

/// The merge rule of each event; an event that only adds context has none beyond the common part.
const MERGERS: [(&str, Merger); 33] = [
    ("PreToolUse", merge_pre_tool_use),
    ("PostToolUse", merge_post_tool_use),
    ("PostToolUseFailure", none),
    ("PostToolBatch", none),
    ("Notification", none),
    ("UserPromptSubmit", merge_user_prompt),
    ("UserPromptExpansion", merge_user_prompt),
    ("SessionStart", merge_session_start),
    ("SessionEnd", none),
    ("Stop", merge_stop),
    ("StopFailure", none),
    ("SubagentStart", none),
    ("SubagentStop", merge_stop),
    ("PreCompact", merge_pre_compact),
    ("PostCompact", none),
    ("PreModelSwitch", merge_pre_model_switch),
    ("PostModelSwitch", none),
    ("PermissionRequest", merge_permission_request),
    ("PermissionDenied", merge_retry),
    ("Setup", none),
    ("TeammateIdle", none),
    ("TaskCreated", none),
    ("TaskCompleted", merge_task_completed),
    ("Elicitation", elicitation),
    ("ElicitationResult", elicitation_result),
    ("ConfigChange", none),
    ("WorktreeCreate", merge_worktree_create),
    ("WorktreeRemove", none),
    ("InstructionsLoaded", none),
    ("CwdChanged", merge_watch_list),
    ("FileChanged", merge_watch_list),
    ("DirectoryAdded", none),
    ("MessageDisplay", merge_display),
];

/// Where the docs say a top-level `decision: "block"` holds the action back ("Decision control"
/// table); the gates and the elicitations read it in their own merge.
const DECISION_BLOCKS: [&str; 10] = [
    "UserPromptSubmit",
    "UserPromptExpansion",
    "PostToolUse",
    "PostToolUseFailure",
    "PostToolBatch",
    "Stop",
    "SubagentStop",
    "ConfigChange",
    "PreCompact",
    "TaskCreated",
];

/// "Returning `decision: \"block\"` or `continue: false` stops the agentic loop": the same block.
const CONTINUE_FALSE_BLOCKS: [&str; 1] = ["PostToolBatch"];

/// Judgment (spec/judgments.md): "`continue: false` ... Takes precedence over any event-specific
/// decision fields", so a `Stop` hook that both stops and blocks the stop stops.
const STOP_OVERRIDES_DECISION: [&str; 2] = ["Stop", "SubagentStop"];

/// Where a hook's `additionalContext` reaches Claude ("Add context for Claude").
const CONTEXT_EVENTS: [&str; 11] = [
    "SessionStart",
    "SubagentStart",
    "UserPromptSubmit",
    "UserPromptExpansion",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "PostToolBatch",
    "Stop",
    "SubagentStop",
    "PostModelSwitch",
];

/// "`additionalContext` ... Ignored when `permissionDecision` is `\"defer\"`."
fn defers(read: &Read) -> bool {
    read.output_field("hookSpecificOutput")
        .and_then(|specific| specific.get("permissionDecision"))
        .and_then(JsValue::as_str)
        == Some("defer")
}

/// Whether a hook's context is left out on its event.
type SkipsContext = fn(&Read) -> bool;

const SKIPS_CONTEXT: [(&str, SkipsContext); 1] = [("PreToolUse", defers)];

/// Whether the event honours the output group `group` (`OUTPUT_CAPS[event].accepts`).
fn accepts(event: &str, group: &str) -> bool {
    output_caps()
        .get(event)
        .is_some_and(|cap| cap.accepts.members.iter().any(|field| field == group))
}

fn read_of(event: &str, tagged: TaggedOutput) -> Read {
    let checked = tagged.output;
    let specific = if accepts(event, "specific") {
        checked
            .as_ref()
            .and_then(|output| output.get("hookSpecificOutput"))
            .filter(|specific| !specific.is_undefined())
            .cloned()
    } else {
        None
    };
    let decides = checked
        .as_ref()
        .and_then(|output| output.get("decision"))
        .and_then(JsValue::as_str)
        == Some("block");
    Read {
        handler: tagged.handler,
        output: checked,
        specific,
        plain_text: tagged.plain_text,
        effects_ignored: tagged.effects_ignored == Some(true),
        blocks: tagged.blocked == Some(true) || decides,
        stdout: tagged.stdout,
    }
}

fn context_of(event: &str, read: &Read) -> Vec<String> {
    let skips = SKIPS_CONTEXT
        .iter()
        .any(|(name, skips)| *name == event && skips(read));
    if !CONTEXT_EVENTS.contains(&event) || skips {
        return vec![];
    }
    let json = text_in(read.specific.as_ref(), "additionalContext");
    let plain = read
        .plain_text
        .as_ref()
        .filter(|plain| plain.kind == PlainTextAs::Context)
        .map(|plain| plain.text.clone());
    [json, plain]
        .into_iter()
        .flatten()
        .filter(|text| !text.is_empty())
        .map(|text| escape_reminder_tags(&text))
        .collect()
}

struct Sequences {
    sequences: Vec<String>,
    issues: Vec<PluginIssue>,
}

fn sequences_of(event: &str, reads: &[Read]) -> Sequences {
    let mut found = Sequences {
        sequences: vec![],
        issues: vec![],
    };
    if !accepts(event, "terminalSequence") {
        return found;
    }
    for read in reads {
        let Some(value) = read.output_field("terminalSequence") else {
            continue;
        };
        match value.as_str().and_then(allowed_terminal_sequence) {
            Some(allowed) => found.sequences.push(allowed),
            None => found.issues.push(warning(
                &format!("hooks.{event}.{}.terminalSequence", read.handler.id),
                "holds something outside the allowlist (OSC 0, 1, 2, 9, 99, 777 and BEL) and is ignored",
            )),
        }
    }
    found
}

/// The hooks whose `continue` is `false`, where the event honours it.
fn stopping_of<'a>(event: &str, reads: &'a [Read]) -> Vec<&'a Read> {
    if !accepts(event, "continue") {
        return vec![];
    }
    reads.iter().filter(|read| read.stops()).collect()
}

fn decision_block_of(event: &str, reads: &[Read], facts: &JsValue) -> Option<MergedBlock> {
    if !DECISION_BLOCKS.contains(&event) || !accepts(event, "decision") {
        return None;
    }
    // "`policy_settings` changes can't be blocked", whatever hook says so.
    if exit2_rule(event)
        .and_then(|rule| rule.cannot_block_when)
        .is_some_and(|cannot| cannot(facts))
    {
        return None;
    }
    let stops = STOP_OVERRIDES_DECISION.contains(&event);
    let found = reads
        .iter()
        .find(|read| read.decides_block() && !(stops && read.stops()))?;
    Some(MergedBlock {
        via: MergedVia::Decision,
        by: found.handler.id.clone(),
        reason: found.output_text("reason"),
    })
}

/// `facts` carry what a rule needs from the event (`source`, `tool_name`).
#[must_use]
pub fn merge_outputs(event: &str, outputs: Vec<TaggedOutput>, facts: &JsValue) -> MergedOutput {
    let reads: Vec<Read> = outputs
        .into_iter()
        .map(|tagged| read_of(event, tagged))
        .collect();
    let stopping = stopping_of(event, &reads);
    let merger = MERGERS
        .iter()
        .find(|(name, _)| *name == event)
        .map_or(none as Merger, |(_, merger)| *merger);
    let merged = merger(&reads, facts);
    let sequences = sequences_of(event, &reads);
    let message_reads: Vec<MessageRead> = reads
        .iter()
        .map(|read| MessageRead {
            output: read.output.clone().map(JsValue::Object),
        })
        .collect();
    let messages = route_messages(event, &message_reads, facts);
    let stop_reason = stopping
        .iter()
        .find_map(|read| read.output_text("stopReason"));
    let block = merged
        .block
        .or_else(|| decision_block_of(event, &reads, facts))
        .or_else(|| {
            let stopper = stopping
                .first()
                .filter(|_| CONTINUE_FALSE_BLOCKS.contains(&event))?;
            Some(MergedBlock {
                via: MergedVia::ContinueFalse,
                by: stopper.handler.id.clone(),
                reason: stop_reason.clone(),
            })
        });
    let mut additional_context: Vec<String> = reads
        .iter()
        .flat_map(|read| context_of(event, read))
        .collect();
    additional_context.extend(messages.to_claude);
    let mut issues = merged.issues;
    issues.extend(sequences.issues);
    MergedOutput {
        block,
        proceeds: stopping.is_empty(),
        stop_reason,
        system_messages: messages.to_user,
        additional_context,
        terminal_sequences: sequences.sequences,
        effects: merged.effects,
        issues,
    }
}

crate::registry::exports! {
    "hooks/output/merge" {
        "mergeOutputs" => ok |event: String, outputs: Vec<TaggedOutput>, facts: JsValue| {
            let facts = if facts.is_undefined() { JsValue::Object(JsObject::new()) } else { facts };
            merge_outputs(&event, outputs, &facts)
        };
    }
}
