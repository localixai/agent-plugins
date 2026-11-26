//! Port of `src/rules/decide/unanswered.ts`, switched.
//!
//! What becomes of a would-be prompt where nobody may answer it. Two facts of the session say so:
//! `cannotPrompt`, a `-p` or SDK run with no permission callback ("Calls needing approval are
//! denied", permission-modes.md), and `permissions.denyInsteadOfPrompting` ("Claude Code denies a
//! tool call that would otherwise wait on a prompt, and the turn continues", settings-reference.md).
//! One rule for both: an `ask` is a denial, every other outcome stands. The setting names its own
//! reason, because its denial carries its own messages.

use serde::{Deserialize, Serialize};

use crate::rules::types::{Decision, DecisionReason};

/// `Pick<DecideContext, 'cannotPrompt' | 'denyInsteadOfPrompting'>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFacts {
    /// Nobody can answer a prompt in this run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cannot_prompt: Option<bool>,
    /// `permissions.denyInsteadOfPrompting` is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deny_instead_of_prompting: Option<bool>,
}

/// The reason a would-be prompt is denied with, or `undefined` where someone may answer it.
fn unanswered_reason(facts: PromptFacts) -> Option<DecisionReason> {
    if facts.deny_instead_of_prompting == Some(true) {
        return Some(DecisionReason::NoApprovalSurface);
    }
    (facts.cannot_prompt == Some(true)).then_some(DecisionReason::PromptUnavailable)
}

/// `UNANSWERED[decision.outcome](decision, reason)`: an `ask` becomes a denial.
const fn unanswered(decision: Decision, reason: DecisionReason) -> Decision {
    match decision {
        Decision::Ask { .. } => Decision::Deny { reason },
        Decision::Allow { .. } | Decision::Deny { .. } | Decision::Classify { .. } => decision,
    }
}

/// The decision once the session's prompt facts apply: an `ask` nobody may answer is denied.
#[must_use]
pub fn settle_prompt(decision: Decision, facts: PromptFacts) -> Decision {
    match unanswered_reason(facts) {
        None => decision,
        Some(reason) => unanswered(decision, reason),
    }
}

/// The denials a `PermissionRequest` hook may still turn into an approval: they stand in for the
/// prompt the hook answers ("Anything that would prompt is denied unless a `PermissionRequest` hook
/// allows it", headless.md; "A `PermissionRequest` hook still answers", settings-reference.md).
pub const PROMPT_DENIALS: [DecisionReason; 2] = [
    DecisionReason::PromptUnavailable,
    DecisionReason::NoApprovalSurface,
];

crate::registry::exports! {
    "rules/decide/unanswered" {
        "settlePrompt" => ok |decision: Decision, facts: PromptFacts| settle_prompt(decision, facts);
        "PROMPT_DENIALS" => ok || crate::registry::Set { members: PROMPT_DENIALS.to_vec() };
    }
}
