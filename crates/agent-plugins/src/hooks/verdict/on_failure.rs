//! Ported from `src/hooks/verdict/on-failure.ts`; only the core uses it now, so it has no wrapper.
//!
//! A hook that fails (hooks.md, "Block the action when a hook fails"): "Each of these counts as a
//! failure": a command hook that can't start, an exit code other than 0 or 2, an HTTP error, a
//! timeout, invalid output. Without `onFailure: "block"` a failure is a non-blocking error and
//! "the action goes ahead"; with it "a failure does what exit code 2 does on that event, except on
//! `PermissionRequest`, where it denies the request".
//!
//! A verdict is a `JsObject` in the package's member order: its `handler` is the host's entry as
//! given.

use super::debug_lines::Subject;
use super::exit2::{Exit2Rule, StderrChannel, exit2_rule};
use super::notices::{
    failed_blocking, handler_label, hook_error, hook_name, timed_out_blocking,
    timed_out_blocks_switch, windows_blocking,
};
use super::route::{BlockText, routed_block, shown_with};
use super::types::{BlockInfo, BlockVia, Feedback, HookNotice};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};

/// What counts as a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureCause {
    /// `cannot-start`.
    CannotStart,
    /// `other-exit`.
    OtherExit,
    /// `http-error`.
    HttpError,
    /// `timeout`.
    Timeout,
    /// `invalid-output`.
    InvalidOutput,
    /// An `mcp_tool` hook's tool answered `isError: true` (no `onFailure` on that type).
    ToolError,
}

/// A failure and its texts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// What failed.
    pub cause: FailureCause,
    /// What the notice of a non-blocking error carries.
    pub text: String,
    /// The hook's own error output, which follows `failed; blocking because ...`.
    pub detail: String,
}

/// The `onFailure` of a `command` or `http` hook; the other types have no such field.
fn on_failure_of(subject: &Subject) -> Option<String> {
    subject
        .handler()
        .get("onFailure")
        .and_then(JsValue::as_str)
        .map(str::to_owned)
}

/// "On Windows without Git Bash, a shell-form `PreToolUse` or `PermissionRequest` command hook
/// that fails with `onFailure` unset can instead block the tool call": without Git Bash "any exit
/// code other than 0 or 2 from that hook counts as a failure". A hook in exec form (`args` set) is
/// not one: "Claude Code then starts the program with no shell".
const WINDOWS_GATES: [&str; 2] = ["PreToolUse", "PermissionRequest"];

fn is_windows_gate(subject: &Subject) -> bool {
    let handler = subject.handler();
    subject.git_bash_missing
        && WINDOWS_GATES.contains(&subject.event.as_str())
        && handler.contains_key("command")
        && handler.get("args").is_none_or(JsValue::is_undefined)
        && on_failure_of(subject).is_none()
}

/// "Background command hooks: command hooks that set `async` or `asyncRewake`": `onFailure` has
/// no effect on them.
fn runs_in_background(subject: &Subject) -> bool {
    let handler = subject.handler();
    let set = |key: &str| handler.get(key).and_then(JsValue::as_bool) == Some(true);
    // `asyncRewake` "implies async" (the SDK type): either field alone makes the hook a background one.
    handler.contains_key("command") && (set("async") || set("asyncRewake"))
}

fn rule_holds(subject: &Subject, test: impl Fn(&Exit2Rule) -> bool) -> bool {
    exit2_rule(&subject.event).is_some_and(|rule| test(&rule))
}

/// `onFailure: "block"` is set and takes effect on this hook: not on the events whose exit 2
/// keeps Claude working (`Stop`, `SubagentStop`, `TaskCompleted`, `TeammateIdle`) and not on
/// background hooks.
#[must_use]
pub fn on_failure_blocks(subject: &Subject) -> bool {
    (on_failure_of(subject).as_deref() == Some("block") || is_windows_gate(subject))
        && !rule_holds(subject, |rule| rule.ignores_on_failure)
        && !runs_in_background(subject)
}

const fn via_of(failure: &Failure) -> BlockVia {
    match failure.cause {
        FailureCause::Timeout => BlockVia::Timeout,
        FailureCause::CannotStart
        | FailureCause::OtherExit
        | FailureCause::HttpError
        | FailureCause::InvalidOutput
        | FailureCause::ToolError => BlockVia::Failure,
    }
}

/// The text of the hook error line.
fn line_of(subject: &Subject, reason: &str) -> BlockText {
    BlockText {
        reason: reason.to_owned(),
        message: hook_error(
            &hook_name(&subject.event, subject.tool_name.as_deref()),
            &handler_label(&subject.handler()),
            reason,
        ),
    }
}

fn block_info(subject: &Subject, failure: &Failure, reason: &str, conditional: bool) -> BlockInfo {
    routed_block(
        &subject.event,
        via_of(failure),
        line_of(subject, reason),
        conditional.then(|| "worktree-directory-remains".to_owned()),
    )
}

/// The text a failure that blocks carries: `failed; blocking because onFailure is "block"` and
/// the detail.
fn blocking_text(subject: &Subject, failure: &Failure) -> String {
    if is_windows_gate(subject) {
        return windows_blocking(&failure.detail);
    }
    if failure.cause == FailureCause::Timeout {
        timed_out_blocking(&failure.detail)
    } else {
        failed_blocking(&failure.detail)
    }
}

fn base(subject: &Subject) -> JsObject {
    JsObject::from_iter([
        ("handler".to_owned(), subject.entry.clone()),
        ("succeeded".to_owned(), JsValue::Bool(false)),
        ("issues".to_owned(), JsValue::Array(vec![])),
    ])
}

/// A verdict's member from a value of the core's.
pub(crate) fn value_of(value: &impl serde::Serialize) -> JsValue {
    to_js_value(value).unwrap_or_default()
}

/// The verdict of a block, with what the docs show of it (`...shownWith(id, block)`).
pub(crate) fn with_block(mut verdict: JsObject, handler_id: &str, block: &BlockInfo) -> JsObject {
    verdict.insert("block".to_owned(), value_of(block));
    if let Ok(JsValue::Object(shown)) = to_js_value(&shown_with(handler_id, block)) {
        verdict.extend(shown);
    }
    verdict
}

/// "On PreModelSwitch, a hook canceled at its timeout blocks the model switch." Judgment: an
/// explicit `onFailure: "continue"` lets it go ahead.
fn blocks_switch(subject: &Subject, failure: &Failure) -> bool {
    rule_holds(subject, |rule| rule.timeout_blocks)
        && failure.cause == FailureCause::Timeout
        && on_failure_of(subject).as_deref() != Some("continue")
}

/// `onFailure: "block"` where the event can be blocked, or where it denies (`PermissionRequest`).
fn blocks_by_on_failure(subject: &Subject) -> bool {
    on_failure_blocks(subject)
        && rule_holds(subject, |rule| {
            (rule.can_block || rule.failure_denies)
                && !rule
                    .cannot_block_when
                    .is_some_and(|cannot| cannot(&subject.facts))
        })
}

/// The kind of block a failure comes to on this event, if any.
fn block_for(subject: &Subject, failure: &Failure) -> Option<BlockInfo> {
    let conditional = rule_holds(subject, |rule| rule.blocks_unless_directory_gone);
    // "Except on events with their own failure contract in the per-event table, such as WorktreeCreate".
    if rule_holds(subject, |rule| rule.any_non_zero_blocks) {
        let reason = if failure.detail.is_empty() {
            &failure.text
        } else {
            &failure.detail
        };
        return Some(block_info(subject, failure, reason, conditional));
    }
    if blocks_by_on_failure(subject) {
        let reason = blocking_text(subject, failure);
        return Some(block_info(subject, failure, &reason, conditional));
    }
    blocks_switch(subject, failure).then(|| {
        let reason = timed_out_blocks_switch(&handler_label(&subject.handler()));
        block_info(subject, failure, &reason, false)
    })
}

/// `onFailure: "block"` on an event where exit 2 only shows stderr (`PostToolUse`): the failure
/// does what exit 2 does there, which is the channel of the table's row.
fn failure_as_exit2_feedback(subject: &Subject, failure: &Failure) -> Option<JsObject> {
    let rule = exit2_rule(&subject.event)?;
    if !on_failure_blocks(subject) || rule.can_block || rule.failure_denies {
        return None;
    }
    let message = line_of(subject, &blocking_text(subject, failure)).message;
    let handler_id = subject.entry_id();
    let mut verdict = base(subject);
    verdict.insert("status".to_owned(), JsValue::from("non-blocking-error"));
    match rule.stderr {
        StderrChannel::Claude => {
            let feedback = Feedback {
                handler_id,
                to: "claude".to_owned(),
                text: message,
            };
            verdict.insert("feedback".to_owned(), value_of(&feedback));
        }
        StderrChannel::User => {
            let notice = HookNotice {
                handler_id,
                message,
            };
            verdict.insert("notice".to_owned(), value_of(&notice));
        }
        StderrChannel::UserAndClaude | StderrChannel::Debug | StderrChannel::Discarded => {}
    }
    Some(verdict)
}

/// What a failed run comes to.
#[must_use]
pub fn failure_verdict(subject: &Subject, failure: &Failure) -> JsObject {
    if let Some(block) = block_for(subject, failure) {
        let mut verdict = base(subject);
        verdict.insert("status".to_owned(), JsValue::from("blocked"));
        return with_block(verdict, &subject.entry_id(), &block);
    }
    if let Some(feedback) = failure_as_exit2_feedback(subject, failure) {
        return feedback;
    }
    let message = line_of(subject, &failure.text).message;
    let status = if failure.cause == FailureCause::Timeout {
        "timed-out"
    } else {
        "non-blocking-error"
    };
    let mut verdict = base(subject);
    verdict.insert("status".to_owned(), JsValue::from(status));
    let notice = HookNotice {
        handler_id: subject.entry_id(),
        message,
    };
    verdict.insert("notice".to_owned(), value_of(&notice));
    verdict
}
