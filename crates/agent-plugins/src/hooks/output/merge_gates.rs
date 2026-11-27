//! Ported from `src/hooks/output/merge-gates.ts`; only the core uses it now, so it has no wrapper.
//!
//! The events that gate an action: `PreToolUse` and `PreModelSwitch` (a `permissionDecision`),
//! and `PermissionRequest` (a `decision` object). hooks.md, "PreToolUse decision control",
//! "PreModelSwitch decision control", "PermissionRequest decision control".

use super::fields::{specific_of, string_in};
use super::merge_rules::{MergedRule, Read, last_of, nothing};
use crate::hooks::verdict::types::{MergedBlock, MergedVia};
use crate::js::{JsObject, JsValue, is_truthy};

/// "When multiple PreToolUse hooks return different decisions, precedence is `deny` > `defer` >
/// `ask` > `allow`."
const PRECEDENCE: [&str; 4] = ["deny", "defer", "ask", "allow"];

struct Gate<'a> {
    read: &'a Read,
    decision: Option<String>,
    reason: Option<String>,
    updated_input: Option<JsValue>,
}

/// The deprecated top-level `decision`: "The deprecated values `approve` and `block` map to
/// `allow` and `deny`".
const DEPRECATED: [(&str, &str); 2] = [("approve", "allow"), ("block", "deny")];

fn decision_of(read: &Read, own: Option<String>) -> Option<String> {
    let top = read.output_text("decision");
    let decision = own.or_else(|| {
        let top = top?;
        DEPRECATED
            .iter()
            .find(|(name, _)| *name == top)
            .map(|(_, permission)| (*permission).to_owned())
    });
    // "A Monitor call reached the hook through a Bash matcher: its `allow` does not apply."
    if read.effects_ignored && decision.as_deref() == Some("allow") {
        None
    } else {
        decision
    }
}

fn member(source: Option<&JsValue>, key: &str) -> Option<JsValue> {
    source?
        .as_object()?
        .get(key)
        .filter(|value| !value.is_undefined())
        .cloned()
}

fn gate_of<'a>(read: &'a Read, event: &str) -> Gate<'a> {
    let own = specific_of(event, read.specific.as_ref());
    let updated_input = if event == "PreToolUse" && !read.effects_ignored {
        member(
            specific_of("PreToolUse", read.specific.as_ref()),
            "updatedInput",
        )
    } else {
        None
    };
    Gate {
        read,
        decision: decision_of(read, string_in(own, "permissionDecision")),
        reason: string_in(own, "permissionDecisionReason").or_else(|| read.output_text("reason")),
        updated_input,
    }
}

fn rank_of(decision: &str) -> usize {
    PRECEDENCE
        .iter()
        .position(|candidate| *candidate == decision)
        .unwrap_or(usize::MAX)
}

/// The winning decision: the first hook that has the highest precedence.
fn winner_of<'g, 'a>(gates: &'g [Gate<'a>]) -> Option<&'g Gate<'a>> {
    gates.iter().fold(None, |best: Option<&Gate<'_>>, gate| {
        match (
            &gate.decision,
            best.and_then(|best| best.decision.as_deref()),
        ) {
            (Some(_), None) => Some(gate),
            (Some(decision), Some(held)) if rank_of(decision) < rank_of(held) => Some(gate),
            _ => best,
        }
    })
}

/// The decisions under which no hook rewrites the call: `updatedInput` is "ignored" for `defer`,
/// the call being resumed later with its own input, and a `deny` runs nothing, as for an exit-2
/// deny (`finalize.ts`). Judgment (spec/judgments.md): a winning `defer` drops the rewrite of every
/// hook.
const DROPS_REWRITE: [&str; 2] = ["deny", "defer"];

/// The rewrite of the call: the last hook's replacement wins (spec/judgments.md).
fn rewrite_of(winner: Option<&Gate<'_>>, gates: &[Gate<'_>]) -> Option<JsValue> {
    let decision = winner.and_then(|winner| winner.decision.as_deref());
    if decision.is_some_and(|decision| DROPS_REWRITE.contains(&decision)) {
        return None;
    }
    let inputs: Vec<JsValue> = gates
        .iter()
        .map(|gate| gate.updated_input.clone().unwrap_or_default())
        .collect();
    last_of(&inputs)
}

fn merge_gates(event: &str, reads: &[Read]) -> MergedRule {
    let gates: Vec<Gate<'_>> = reads.iter().map(|read| gate_of(read, event)).collect();
    let winner = winner_of(&gates);
    let updated_input = rewrite_of(winner, &gates);
    let mut rewrite = JsObject::new();
    if let Some(input) = updated_input {
        rewrite.insert("updatedInput".to_owned(), input);
    }
    let Some((winner, decision)) =
        winner.and_then(|winner| Some((winner, winner.decision.clone()?)))
    else {
        return MergedRule {
            effects: rewrite,
            issues: vec![],
            block: None,
        };
    };
    let reason = winner.reason.clone();
    let mut effects = JsObject::new();
    effects.insert(
        "permissionDecision".to_owned(),
        JsValue::from(decision.as_str()),
    );
    if let Some(reason) = &reason {
        effects.insert(
            "permissionDecisionReason".to_owned(),
            JsValue::from(reason.as_str()),
        );
    }
    effects.extend(rewrite);
    let block = (decision == "deny").then(|| MergedBlock {
        via: MergedVia::Decision,
        by: winner.read.handler.id.clone(),
        reason,
    });
    MergedRule {
        effects,
        issues: vec![],
        block,
    }
}

/// `PreToolUse`.
#[must_use]
pub fn merge_pre_tool_use(reads: &[Read], _facts: &JsValue) -> MergedRule {
    merge_gates("PreToolUse", reads)
}

/// `PreModelSwitch`.
#[must_use]
pub fn merge_pre_model_switch(reads: &[Read], _facts: &JsValue) -> MergedRule {
    merge_gates("PreModelSwitch", reads)
}

fn decision_of_request(read: &Read) -> Option<JsObject> {
    member(
        specific_of("PermissionRequest", read.specific.as_ref()),
        "decision",
    )
    .and_then(|decision| decision.as_object().cloned())
}

fn behavior_of(decision: &JsObject) -> Option<&str> {
    decision.get("behavior").and_then(JsValue::as_str)
}

/// The denials' answer: the first `message`, and `interrupt` when any denying hook sets it.
fn denied(denies: &[(&Read, JsObject)], first: &Read) -> MergedRule {
    let message = denies
        .iter()
        .find_map(|(_, decision)| decision.get("message").filter(|text| !text.is_undefined()))
        .cloned();
    let interrupt = denies
        .iter()
        .any(|(_, decision)| decision.get("interrupt").is_some_and(is_truthy));
    let mut request = JsObject::from_iter([("behavior".to_owned(), JsValue::from("deny"))]);
    if let Some(message) = &message {
        request.insert("message".to_owned(), message.clone());
    }
    if interrupt {
        request.insert("interrupt".to_owned(), JsValue::Bool(true));
    }
    MergedRule {
        effects: JsObject::from_iter([("permissionRequest".to_owned(), JsValue::Object(request))]),
        issues: vec![],
        block: Some(MergedBlock {
            via: MergedVia::Decision,
            by: first.handler.id.clone(),
            reason: message.and_then(|message| message.as_str().map(str::to_owned)),
        }),
    }
}

/// Judgment (spec/judgments.md): the docs give no precedence across hooks for
/// `PermissionRequest`: a deny wins over an allow, the `updatedPermissions` of the allowing hooks
/// are concatenated, the last `updatedInput` wins, and a deny's `interrupt` counts when any denying
/// hook sets it.
#[must_use]
pub fn merge_permission_request(reads: &[Read], _facts: &JsValue) -> MergedRule {
    let decisions: Vec<(&Read, JsObject)> = reads
        .iter()
        .filter_map(|read| decision_of_request(read).map(|decision| (read, decision)))
        .collect();
    let denies: Vec<(&Read, JsObject)> = decisions
        .iter()
        .filter(|(_, decision)| behavior_of(decision) == Some("deny"))
        .cloned()
        .collect();
    if let Some((first, _)) = denies.first() {
        return denied(&denies, first);
    }
    // "A Monitor call reached the hook through a Bash matcher": its `allow` and `updatedInput` do
    // not apply.
    let allows: Vec<&JsObject> = decisions
        .iter()
        .filter(|(read, decision)| behavior_of(decision) == Some("allow") && !read.effects_ignored)
        .map(|(_, decision)| decision)
        .collect();
    if allows.is_empty() {
        return nothing().into();
    }
    let inputs: Vec<JsValue> = allows
        .iter()
        .map(|allow| allow.get("updatedInput").cloned().unwrap_or_default())
        .collect();
    let updated_input = last_of(&inputs);
    let updated_permissions: Vec<JsValue> = allows
        .iter()
        .flat_map(|allow| {
            allow
                .get("updatedPermissions")
                .and_then(JsValue::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    let mut request = JsObject::from_iter([("behavior".to_owned(), JsValue::from("allow"))]);
    if let Some(input) = updated_input {
        request.insert("updatedInput".to_owned(), input);
    }
    if !updated_permissions.is_empty() {
        request.insert(
            "updatedPermissions".to_owned(),
            JsValue::Array(updated_permissions),
        );
    }
    MergedRule {
        effects: JsObject::from_iter([("permissionRequest".to_owned(), JsValue::Object(request))]),
        issues: vec![],
        block: None,
    }
}
