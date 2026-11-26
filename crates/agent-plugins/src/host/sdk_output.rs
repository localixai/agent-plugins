//! Port of `src/host/sdk-output.ts`, switched.
//!
//! A merged `HookOutcome` as the JSON a Claude Agent SDK hook callback returns (agent-sdk/hooks.md,
//! "Outputs": "SDK callback hooks use the same JSON output format as Claude Code shell command
//! hooks"). The kit has already merged the plugins' hooks of the event into one outcome; this writes
//! that outcome back as one hook's answer, each field typed by the SDK's own output types.
//!
//! Judgment (spec/judgments.md): several hooks' `additionalContext` and `systemMessage` texts are
//! joined with a newline, terminal sequences with nothing; what the SDK output has no field for
//! (`PreCompact` instructions, background deliveries, notices) stays on the outcome for the host.
//!
//! An event the table does not name has no `hookSpecificOutput` (the TypeScript's table covers the
//! official union, so no outcome names another).

use serde::Deserialize;

use crate::format::defined::defined;
use crate::hooks::output::worktree_path::{WorktreeAnswer, WorktreeEffects, worktree_candidate};
use crate::js::{JsObject, JsValue};

/// The part of a block an SDK answer reads.
#[derive(Debug, Clone, Deserialize)]
pub struct OutcomeBlock {
    /// How the hook blocked.
    pub via: String,
    /// The text of the block.
    #[serde(default)]
    pub reason: String,
}

/// The part of a merged `HookOutcome` an SDK answer reads.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnyOutcome {
    /// The event.
    pub event: String,
    /// `false`: the guarded action must not happen.
    #[serde(default)]
    pub proceed: bool,
    /// The block, when a hook blocked.
    #[serde(default)]
    pub block: Option<OutcomeBlock>,
    /// `continue`.
    #[serde(rename = "continue")]
    pub proceeds: bool,
    /// `stopReason`.
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// The `systemMessage` texts.
    #[serde(default)]
    pub system_messages: Vec<String>,
    /// The `additionalContext` texts.
    #[serde(default)]
    pub additional_context: Vec<String>,
    /// The merged effects.
    #[serde(default)]
    pub effects: JsObject,
    /// The terminal sequences.
    #[serde(default)]
    pub terminal_sequences: Vec<String>,
    /// The stdout of each `WorktreeCreate` command hook that exited 0.
    #[serde(default)]
    pub stdout: Vec<String>,
}

fn joined(texts: &[String], separator: &str) -> JsValue {
    if texts.is_empty() {
        JsValue::Undefined
    } else {
        JsValue::String(texts.join(separator))
    }
}

fn context_of(outcome: &AnyOutcome) -> JsValue {
    joined(&outcome.additional_context, "\n")
}

/// A member of the effects, `undefined` when absent.
fn effect(outcome: &AnyOutcome, key: &str) -> JsValue {
    outcome
        .effects
        .get(key)
        .cloned()
        .unwrap_or(JsValue::Undefined)
}

/// `first ?? second`.
fn or_else(first: JsValue, second: impl FnOnce() -> JsValue) -> JsValue {
    if first.is_nullish() { second() } else { first }
}

/// A hook held the action back by its decision or exit code, not by `continue: false`.
fn blocks_by(outcome: &AnyOutcome) -> bool {
    outcome
        .block
        .as_ref()
        .is_some_and(|block| block.via != "continue-false")
}

/// The text of a block `blocksBy` counts, when it has one (it is empty when routed to no one).
fn reason_of(outcome: &AnyOutcome) -> JsValue {
    match &outcome.block {
        Some(block) if blocks_by(outcome) && !block.reason.is_empty() => {
            JsValue::String(block.reason.clone())
        }
        _ => JsValue::Undefined,
    }
}

fn specific(event: &str, fields: Vec<(&str, JsValue)>) -> JsValue {
    let mut object = JsObject::new();
    object.insert(
        "hookEventName".to_owned(),
        JsValue::String(event.to_owned()),
    );
    let given: JsObject = fields
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    object.extend(defined(given));
    JsValue::Object(object)
}

/// An event whose `hookSpecificOutput` has only `additionalContext`.
fn context_only(event: &str, outcome: &AnyOutcome) -> JsValue {
    specific(event, vec![("additionalContext", context_of(outcome))])
}

/// A blocking hook (exit 2) on a gate is a deny, as `permissionDecision` says it.
fn gate_decision(outcome: &AnyOutcome) -> JsValue {
    or_else(effect(outcome, "permissionDecision"), || {
        if blocks_by(outcome) {
            JsValue::String("deny".to_owned())
        } else {
            JsValue::Undefined
        }
    })
}

/// `PreModelSwitch` takes no `defer`.
fn model_decision(decision: &JsValue) -> JsValue {
    match decision.as_str() {
        Some(kept @ ("allow" | "deny" | "ask")) => JsValue::String(kept.to_owned()),
        _ => JsValue::Undefined,
    }
}

fn watch_paths_of(outcome: &AnyOutcome) -> JsValue {
    effect(outcome, "watchPaths")
}

fn permission_request_of(outcome: &AnyOutcome) -> JsValue {
    let decision = or_else(effect(outcome, "permissionRequest"), || {
        if !blocks_by(outcome) {
            return JsValue::Undefined;
        }
        let message = or_else(reason_of(outcome), || JsValue::String(String::new()));
        let mut deny = JsObject::new();
        deny.insert("behavior".to_owned(), JsValue::String("deny".to_owned()));
        deny.insert("message".to_owned(), message);
        JsValue::Object(deny)
    });
    if decision.is_undefined() {
        return JsValue::Undefined;
    }
    let mut object = JsObject::new();
    object.insert(
        "hookEventName".to_owned(),
        JsValue::String("PermissionRequest".to_owned()),
    );
    object.insert("decision".to_owned(), decision);
    JsValue::Object(object)
}

fn worktree_of(outcome: &AnyOutcome) -> JsValue {
    let worktree_path = effect(outcome, "worktreePath").as_str().map(str::to_owned);
    let answer = WorktreeAnswer {
        proceed: outcome.proceed,
        stdout: outcome.stdout.clone(),
        effects: WorktreeEffects { worktree_path },
    };
    worktree_candidate(&answer).map_or(JsValue::Undefined, |worktree_path| {
        let mut object = JsObject::new();
        object.insert(
            "hookEventName".to_owned(),
            JsValue::String("WorktreeCreate".to_owned()),
        );
        object.insert("worktreePath".to_owned(), JsValue::String(worktree_path));
        JsValue::Object(object)
    })
}

fn elicitation_of(event: &str, outcome: &AnyOutcome) -> JsValue {
    let elicitation = effect(outcome, "elicitation");
    let member = |key: &str| elicitation.get(key).cloned().unwrap_or(JsValue::Undefined);
    specific(
        event,
        vec![("action", member("action")), ("content", member("content"))],
    )
}

fn pre_tool_use(outcome: &AnyOutcome) -> JsValue {
    specific(
        "PreToolUse",
        vec![
            ("permissionDecision", gate_decision(outcome)),
            (
                "permissionDecisionReason",
                or_else(effect(outcome, "permissionDecisionReason"), || {
                    reason_of(outcome)
                }),
            ),
            ("updatedInput", effect(outcome, "updatedInput")),
            ("additionalContext", context_of(outcome)),
        ],
    )
}

fn pre_model_switch(outcome: &AnyOutcome) -> JsValue {
    let decision = gate_decision(outcome);
    specific(
        "PreModelSwitch",
        vec![
            ("permissionDecision", model_decision(&decision)),
            (
                "permissionDecisionReason",
                or_else(effect(outcome, "permissionDecisionReason"), || {
                    reason_of(outcome)
                }),
            ),
        ],
    )
}

/// Every official event, so an event the SDK adds is written here.
fn specific_of(outcome: &AnyOutcome) -> JsValue {
    let event = outcome.event.as_str();
    let fields = |names: &[&'static str]| -> Vec<(&'static str, JsValue)> {
        names
            .iter()
            .map(|name| (*name, effect(outcome, name)))
            .collect()
    };
    match event {
        "PreToolUse" => pre_tool_use(outcome),
        "PreModelSwitch" => pre_model_switch(outcome),
        "PermissionRequest" => permission_request_of(outcome),
        "PostToolUse" => specific(event, {
            let mut all = vec![("additionalContext", context_of(outcome))];
            all.extend(fields(&["classifierContext", "updatedToolOutput"]));
            all
        }),
        "SessionStart" => specific(
            event,
            vec![
                ("additionalContext", context_of(outcome)),
                ("initialUserMessage", effect(outcome, "initialUserMessage")),
                ("sessionTitle", effect(outcome, "sessionTitle")),
                ("watchPaths", watch_paths_of(outcome)),
                ("reloadSkills", effect(outcome, "reloadSkills")),
            ],
        ),
        "UserPromptSubmit" => specific(event, {
            let mut all = vec![("additionalContext", context_of(outcome))];
            all.extend(fields(&["sessionTitle", "suppressOriginalPrompt"]));
            all
        }),
        "UserPromptExpansion" => specific(event, {
            let mut all = vec![("additionalContext", context_of(outcome))];
            all.extend(fields(&["suppressOriginalPrompt"]));
            all
        }),
        "CwdChanged" | "FileChanged" => {
            specific(event, vec![("watchPaths", watch_paths_of(outcome))])
        }
        "MessageDisplay" => specific(event, fields(&["displayContent"])),
        "PermissionDenied" => specific(event, fields(&["retry"])),
        "Elicitation" | "ElicitationResult" => elicitation_of(event, outcome),
        "WorktreeCreate" => worktree_of(outcome),
        "Notification" | "PostModelSwitch" | "PostToolBatch" | "PostToolUseFailure" | "Setup"
        | "Stop" | "SubagentStart" | "SubagentStop" => context_only(event, outcome),
        _ => JsValue::Undefined,
    }
}

/// The outcome of the plugins' hooks of one event as the answer of one SDK hook callback.
#[must_use]
pub fn to_sdk_hook_output(outcome: &AnyOutcome) -> JsObject {
    let text = |value: &Option<String>| value.clone().map_or(JsValue::Undefined, JsValue::String);
    let fields: [(&str, JsValue); 7] = [
        (
            "continue",
            if outcome.proceeds {
                JsValue::Undefined
            } else {
                JsValue::Bool(false)
            },
        ),
        ("stopReason", text(&outcome.stop_reason)),
        ("systemMessage", joined(&outcome.system_messages, "\n")),
        ("terminalSequence", joined(&outcome.terminal_sequences, "")),
        (
            "decision",
            if blocks_by(outcome) {
                JsValue::String("block".to_owned())
            } else {
                JsValue::Undefined
            },
        ),
        ("reason", reason_of(outcome)),
        ("hookSpecificOutput", specific_of(outcome)),
    ];
    defined(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

crate::registry::exports! {
    "host/sdk-output" {
        "toSdkHookOutput" => ok |outcome: AnyOutcome| to_sdk_hook_output(&outcome);
    }
}
