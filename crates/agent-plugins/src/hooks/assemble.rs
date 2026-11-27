//! Port of `src/hooks/assemble.ts`, switched.
//!
//! What an event came to once every handler's run is read and the outputs merged (docs/API.md,
//! "Hooks"). A block of an exit code (exit 2, a failure) wins over a block of a JSON decision, in
//! whichever handler it sits: an exit-2 block's "fields ... can't override the block". Among exit
//! blocks the first in handler order wins; a JSON decision of that same hook speaks for it ("If
//! your hook also printed JSON that makes a blocking decision, Claude Code uses that decision's
//! reason instead"). Judgment: spec/judgments.md.
//!
//! A verdict is the host's whole value (its handler, its background promise), so the outcome
//! carries its members as given.

use serde::Deserialize;

use super::output::finalize::finalize_effects;
use super::select::once::{OnceEntry, consumed_by_run};
use super::source_kind::{LabelledEntry, is_plugin_source, plugin_label};
use super::verdict::notices::handler_label;
use super::verdict::route::{BlockText, routed_block};
use super::verdict::types::{BlockInfo, BlockRoute, BlockVia, MergedOutput, MergedVia};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};

type Blocked = Option<JsObject>;

/// Only a blocking decision's reason replaces the stderr; the `reason` of `continue: false` is a
/// stop reason.
const fn replaces_reason(via: MergedVia) -> bool {
    match via {
        MergedVia::Decision => true,
        MergedVia::ContinueFalse => false,
    }
}

/// A merged block's way of holding back, as a block of the outcome names it.
const fn block_via(via: MergedVia) -> BlockVia {
    match via {
        MergedVia::Decision => BlockVia::Decision,
        MergedVia::ContinueFalse => BlockVia::ContinueFalse,
    }
}

/// The reason a merged block of the same hook gives in place of the stderr, if the docs let it.
fn replacement(merged: &MergedOutput, handler_id: &str) -> Option<String> {
    let block = merged.block.as_ref()?;
    if block.by == handler_id && replaces_reason(block.via) {
        block.reason.clone()
    } else {
        None
    }
}

fn object(value: &impl serde::Serialize) -> JsObject {
    match to_js_value(value) {
        Ok(JsValue::Object(members)) => members,
        _ => JsObject::new(),
    }
}

fn member<'a>(verdict: &'a JsObject, key: &str) -> Option<&'a JsValue> {
    verdict.get(key).filter(|value| !value.is_undefined())
}

fn handler_id_of(verdict: &JsObject) -> String {
    member(verdict, "handler")
        .and_then(|handler| handler.get("id"))
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn with_by(mut block: JsObject, by: String) -> JsObject {
    block.insert("by".to_owned(), JsValue::from(by));
    block
}

/// The verdict-level block of the first hook that has one, its reason replaced by its own JSON
/// decision's.
fn exit_block(event: &str, verdicts: &[JsObject], merged: &MergedOutput) -> Blocked {
    let found = verdicts
        .iter()
        .find(|verdict| member(verdict, "block").is_some())?;
    let given = member(found, "block")?.as_object()?.clone();
    let handler_id = handler_id_of(found);
    let info = BlockInfo::deserialize(JsValue::Object(given.clone())).ok();
    let decision = replacement(merged, &handler_id);
    let block = match (decision, info) {
        (Some(decision), Some(info)) if info.to != BlockRoute::None => object(&routed_block(
            event,
            info.via,
            BlockText {
                reason: decision.clone(),
                message: decision,
            },
            info.condition,
        )),
        _ => given,
    };
    Some(with_by(block, handler_id))
}

fn decision_block(event: &str, merged: &MergedOutput) -> Blocked {
    let block = merged.block.as_ref()?;
    let reason = block.reason.clone().unwrap_or_default();
    let routed = routed_block(
        event,
        block_via(block.via),
        BlockText {
            reason: reason.clone(),
            message: reason,
        },
        None,
    );
    Some(with_by(object(&routed), block.by.clone()))
}

/// The facts of a verdict's run (`RunFacts`); a verdict the run stage did not time (one built by
/// hand) still names its handler and plugin, and says it finished.
#[must_use]
pub fn run_facts_of_verdict(verdict: &JsObject) -> JsObject {
    if let Some(JsValue::Object(ran)) = member(verdict, "ran") {
        return ran.clone();
    }
    let entry = member(verdict, "handler").cloned().unwrap_or_default();
    let plugin = LabelledEntry::deserialize(entry.clone())
        .ok()
        .filter(|entry| is_plugin_source(&entry.source))
        .and_then(|entry| plugin_label(&entry));
    let handler = entry
        .get("handler")
        .and_then(JsValue::as_object)
        .cloned()
        .unwrap_or_default();
    let mut facts =
        JsObject::from_iter([("handler".to_owned(), JsValue::from(handler_label(&handler)))]);
    if let Some(plugin) = plugin {
        facts.insert("plugin".to_owned(), JsValue::from(plugin));
    }
    facts.insert("ending".to_owned(), JsValue::from("finished"));
    facts
}

fn present(verdicts: &[JsObject], key: &str) -> JsValue {
    JsValue::Array(
        verdicts
            .iter()
            .filter_map(|verdict| member(verdict, key).cloned())
            .collect(),
    )
}

fn run_of(verdict: &JsObject) -> JsValue {
    let mut run = run_facts_of_verdict(verdict);
    if let Some(exit_code) = member(verdict, "exitCode") {
        run.insert("exitCode".to_owned(), exit_code.clone());
    }
    run.insert(
        "handlerId".to_owned(),
        JsValue::from(handler_id_of(verdict)),
    );
    run.insert(
        "status".to_owned(),
        member(verdict, "status").cloned().unwrap_or_default(),
    );
    if let Some(via) = member(verdict, "block").and_then(|block| block.get("via")) {
        run.insert("via".to_owned(), via.clone());
    }
    JsValue::Object(run)
}

fn consumed_once(verdicts: &[JsObject]) -> JsValue {
    JsValue::Array(
        verdicts
            .iter()
            .filter(|verdict| {
                let succeeded = member(verdict, "succeeded").and_then(JsValue::as_bool);
                member(verdict, "handler")
                    .and_then(|entry| OnceEntry::deserialize(entry.clone()).ok())
                    .is_some_and(|entry| consumed_by_run(&entry, succeeded == Some(true)))
            })
            .map(|verdict| JsValue::from(handler_id_of(verdict)))
            .collect(),
    )
}

fn strings(list: Vec<String>) -> JsValue {
    JsValue::Array(list.into_iter().map(JsValue::from).collect())
}

fn all_issues(mut all: Vec<JsValue>, merged: &MergedOutput, verdicts: &[JsObject]) -> JsValue {
    all.extend(
        merged
            .issues
            .iter()
            .map(|issue| to_js_value(issue).unwrap_or_default()),
    );
    for verdict in verdicts {
        if let Some(JsValue::Array(own)) = member(verdict, "issues") {
            all.extend(own.iter().cloned());
        }
    }
    JsValue::Array(all)
}

/// The outcome of `event` from its verdicts and the merge of their outputs.
#[must_use]
pub fn assemble(
    event: &str,
    verdicts: &[JsObject],
    merged: &MergedOutput,
    issues: Vec<JsValue>,
) -> JsObject {
    let block = exit_block(event, verdicts, merged).or_else(|| decision_block(event, merged));
    let stop = verdicts
        .iter()
        .find_map(|verdict| member(verdict, "stop").cloned());
    let stop_reason = merged.stop_reason.clone().or_else(|| {
        stop.as_ref()
            .and_then(|stop| stop.get("reason"))
            .and_then(JsValue::as_str)
            .map(str::to_owned)
    });
    let info = block
        .as_ref()
        .and_then(|block| BlockInfo::deserialize(JsValue::Object(block.clone())).ok());
    let effects = finalize_effects(event, merged.effects.clone(), info.as_ref());
    let mut outcome = JsObject::from_iter([
        ("event".to_owned(), JsValue::from(event)),
        ("proceed".to_owned(), JsValue::Bool(block.is_none())),
    ]);
    if let Some(block) = block {
        outcome.insert("block".to_owned(), JsValue::Object(block));
    }
    outcome.insert(
        "continue".to_owned(),
        JsValue::Bool(merged.proceeds && stop.is_none()),
    );
    if let Some(reason) = stop_reason {
        outcome.insert("stopReason".to_owned(), JsValue::from(reason));
    }
    let runs = JsValue::Array(verdicts.iter().map(run_of).collect());
    outcome.extend([
        (
            "systemMessages".to_owned(),
            strings(merged.system_messages.clone()),
        ),
        (
            "additionalContext".to_owned(),
            strings(merged.additional_context.clone()),
        ),
        ("effects".to_owned(), JsValue::Object(effects)),
        (
            "terminalSequences".to_owned(),
            strings(merged.terminal_sequences.clone()),
        ),
        ("notices".to_owned(), present(verdicts, "notice")),
        ("feedback".to_owned(), present(verdicts, "feedback")),
        ("stdout".to_owned(), present(verdicts, "stdout")),
        ("runs".to_owned(), runs),
        ("consumedOnce".to_owned(), consumed_once(verdicts)),
        ("background".to_owned(), present(verdicts, "delivery")),
        ("issues".to_owned(), all_issues(issues, merged, verdicts)),
    ]);
    outcome
}

/// The outcome of an event no handler ran for: nothing is held back.
#[must_use]
pub fn idle_outcome(event: &str, issues: Vec<JsValue>) -> JsObject {
    let empty = || JsValue::Array(vec![]);
    JsObject::from_iter([
        ("event".to_owned(), JsValue::from(event)),
        ("proceed".to_owned(), JsValue::Bool(true)),
        ("continue".to_owned(), JsValue::Bool(true)),
        ("systemMessages".to_owned(), empty()),
        ("additionalContext".to_owned(), empty()),
        ("effects".to_owned(), JsValue::Object(JsObject::new())),
        ("terminalSequences".to_owned(), empty()),
        ("notices".to_owned(), empty()),
        ("feedback".to_owned(), empty()),
        ("stdout".to_owned(), empty()),
        ("runs".to_owned(), empty()),
        ("consumedOnce".to_owned(), empty()),
        ("background".to_owned(), empty()),
        ("issues".to_owned(), JsValue::Array(issues)),
    ])
}

crate::registry::exports! {
    "hooks/assemble" {
        "runFactsOfVerdict" => ok |verdict: JsObject| run_facts_of_verdict(&verdict);
        "assemble" => ok |event: String, verdicts: Vec<JsObject>, merged: MergedOutput, issues: Vec<JsValue>|
            assemble(&event, &verdicts, &merged, issues);
        "idleOutcome" => ok |event: String, issues: Vec<JsValue>| idle_outcome(&event, issues);
    }
}
