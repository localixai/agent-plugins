//! Ported from `src/hooks/verdict/model-verdict.ts`; only the core uses it now, so it has no wrapper.
//!
//! A `prompt` or `agent` hook's verdict (hooks.md, "Response schema" and "Agent hook
//! configuration"). What `ok: false` does on each event is the table of `remote/ok-false.ts`; this
//! turns its answer into the verdict: the reason blocks the action, ends the turn, or goes to
//! Claude.

use super::debug_lines::Subject;
use super::exit2::{StderrChannel, exit2_rule, ignored};
use super::notices::{handler_label, hook_error, hook_name};
use super::on_failure::{Failure, FailureCause, failure_verdict, value_of, with_block};
use super::route::{BlockText, routed_block};
use super::types::{BlockVia, Feedback};
use crate::hooks::handlers::remote::ok_false::{
    OkFalseEffect, OkFalseOptions, ok_false_effect, parse_event,
};
use crate::hooks::handlers::remote::types::ModelRun;
use crate::js::{JsObject, JsValue};

/// "Agent hooks and other events ignore `impossible`", and an agent has no `continueOnBlock`.
fn is_agent(handler_type: &str) -> bool {
    const IS_AGENT: [(&str, bool); 5] = [
        ("command", false),
        ("http", false),
        ("mcp_tool", false),
        ("prompt", false),
        ("agent", true),
    ];
    IS_AGENT
        .iter()
        .any(|(name, agent)| *name == handler_type && *agent)
}

fn continue_on_block(subject: &Subject) -> Option<bool> {
    subject
        .handler()
        .get("continueOnBlock")
        .and_then(JsValue::as_bool)
}

fn common(subject: &Subject) -> JsObject {
    JsObject::from_iter([
        ("handler".to_owned(), subject.entry.clone()),
        ("issues".to_owned(), JsValue::Array(vec![])),
    ])
}

/// What `ok: false` came to on an event that has a verdict for it.
fn failed_verdict(subject: &Subject, reason: &str, ends_turn: bool) -> JsObject {
    let message = hook_error(
        &hook_name(&subject.event, subject.tool_name.as_deref()),
        &handler_label(&subject.handler()),
        reason,
    );
    let rule = exit2_rule(&subject.event).unwrap_or(ignored(StderrChannel::Discarded));
    // A teammate that stops is not an action held back: on `TeammateIdle` ending the turn is the
    // reverse of a block ("by default the teammate stops"; `continueOnBlock` "keeps it working"),
    // and `TaskCompleted` fired by a teammate stopping "behaves like `TeammateIdle`".
    let stops_teammate = ends_turn && rule.stops_teammate;
    let holds = rule.can_block && !stops_teammate;
    let to_claude = !ends_turn && !rule.can_block;
    let mut verdict = common(subject);
    let status = if holds {
        "blocked"
    } else {
        "non-blocking-error"
    };
    verdict.insert("status".to_owned(), JsValue::from(status));
    verdict.insert("succeeded".to_owned(), JsValue::Bool(false));
    if holds {
        let block = routed_block(
            &subject.event,
            BlockVia::Decision,
            BlockText {
                reason: reason.to_owned(),
                message: message.clone(),
            },
            None,
        );
        verdict = with_block(verdict, &subject.entry_id(), &block);
    }
    if ends_turn {
        let stop = JsObject::from_iter([("reason".to_owned(), JsValue::from(reason))]);
        verdict.insert("stop".to_owned(), JsValue::Object(stop));
    }
    if to_claude {
        let feedback = Feedback {
            handler_id: subject.entry_id(),
            to: "claude".to_owned(),
            text: message,
        };
        verdict.insert("feedback".to_owned(), value_of(&feedback));
    }
    verdict
}

/// What a model's answer comes to.
#[must_use]
pub fn interpret_model(subject: &Subject, run: &ModelRun) -> JsObject {
    let mut verdict = common(subject);
    let Some(event) = parse_event(&subject.event) else {
        verdict.insert("status".to_owned(), JsValue::from("success"));
        verdict.insert("succeeded".to_owned(), JsValue::Bool(run.ok));
        return verdict;
    };
    let handler_type = subject
        .handler()
        .get("type")
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned();
    let effect = ok_false_effect(
        event,
        run,
        OkFalseOptions {
            agent: is_agent(&handler_type),
            continue_on_block: continue_on_block(subject),
            // The input does not say what fired `TaskCompleted`: the host does
            // (`DispatchRequest.taskCompletedBy`).
            teammate_stopped: Some(subject.teammate_stopped),
        },
    );
    if let OkFalseEffect::Block {
        reason, ends_turn, ..
    } = effect
    {
        return failed_verdict(subject, &reason, ends_turn);
    }
    verdict.insert("status".to_owned(), JsValue::from("success"));
    verdict.insert("succeeded".to_owned(), JsValue::Bool(true));
    verdict
}

/// The model did not answer, or answered something that is not the response schema.
#[must_use]
pub fn interpret_model_failed(subject: &Subject, message: &str) -> JsObject {
    failure_verdict(
        subject,
        &Failure {
            cause: FailureCause::ToolError,
            text: message.to_owned(),
            detail: message.to_owned(),
        },
    )
}
