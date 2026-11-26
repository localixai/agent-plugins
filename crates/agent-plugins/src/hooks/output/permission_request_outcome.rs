//! Port of `src/hooks/output/permission-request-outcome.ts`, switched.

use serde::Deserialize;

use crate::rules::decide::unanswered::PROMPT_DENIALS;
use crate::rules::types::{Decision, DecisionReason};

/// `HookSpecificOutputOf<'PermissionRequest'>['decision']['behavior']`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestBehavior {
    /// The hook allows the call.
    Allow,
    /// The hook denies it.
    Deny,
}

/// The hook's answer, as far as the decision reads it.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct RequestAnswer {
    /// What the hook decided.
    pub behavior: RequestBehavior,
}

/// The effects of a merged `PermissionRequest` outcome the decision reads.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEffects {
    /// The hooks' answer.
    #[serde(default)]
    pub permission_request: Option<RequestAnswer>,
}

/// `Pick<HookOutcome<'PermissionRequest'>, 'effects'>`.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct RequestOutcome {
    /// The merged effects.
    pub effects: RequestEffects,
}

/// What the hook's answer makes of the call: it runs, or it is denied by the hook.
const fn answer_of(behavior: RequestBehavior) -> Decision {
    match behavior {
        RequestBehavior::Allow => Decision::Allow {
            reason: DecisionReason::Hook,
        },
        RequestBehavior::Deny => Decision::Deny {
            reason: DecisionReason::Hook,
        },
    }
}

/// Which outcomes are a prompt the hooks answer; a denial counts only in place of a prompt.
const fn prompts(decision: Decision) -> bool {
    matches!(decision, Decision::Ask { .. })
}

const fn reason_of(decision: Decision) -> DecisionReason {
    match decision {
        Decision::Allow { reason }
        | Decision::Ask { reason }
        | Decision::Deny { reason }
        | Decision::Classify { reason, .. } => reason,
    }
}

/// A decision that stands where a prompt would be: an `ask`, or a denial in place of a prompt nobody may answer.
fn answerable(decision: Decision) -> bool {
    prompts(decision) || PROMPT_DENIALS.contains(&reason_of(decision))
}

/// The decision once the `PermissionRequest` hooks have answered (hooks.md, "PermissionRequest
/// decision control"). The hooks answer the prompt a call would get, and also where no prompt may be
/// shown: "Anything that would prompt is denied unless a `PermissionRequest` hook allows it"
/// (headless.md), and with `permissions.denyInsteadOfPrompting` "A `PermissionRequest` hook still
/// answers" (settings-reference.md). A call denied before any hook is asked (a deny rule, a
/// critical-path removal in auto mode) and a call already decided stay as they are; with no answer
/// the decision stands. The `updatedInput` and `updatedPermissions` of an allow stay in `effects`.
#[must_use]
pub fn permission_request_decision(decision: Decision, outcome: &RequestOutcome) -> Decision {
    match outcome.effects.permission_request {
        Some(answer) if answerable(decision) => answer_of(answer.behavior),
        _ => decision,
    }
}

crate::registry::exports! {
    "hooks/output/permission-request-outcome" {
        "permissionRequestDecision" => ok |decision: Decision, outcome: RequestOutcome|
            permission_request_decision(decision, &outcome);
    }
}
