//! Port of `src/hooks/output/pre-tool-use-outcome.ts`, switched.
//!
//! What the permission check (`decide`) reads of the `PreToolUse` hooks: their merged
//! `permissionDecision`, whether a hook exited with code 2 ("stops the tool call before permission
//! rules are evaluated"), and whether the answer carries `updatedInput`, which `AskUserQuestion` and
//! `ExitPlanMode` need beside an `allow` (hooks.md, "Tools that require user interaction").

use serde::{Deserialize, Serialize};

use crate::js::JsValue;

/// The merged block of the outcome (`HookOutcome.block`), by how it was decided.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct OutcomeBlock {
    /// How the action was held back.
    #[serde(default)]
    pub via: Option<String>,
}

/// The merged effects of the outcome (`HookOutcome.effects`): the two fields the check reads.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeEffects {
    /// The merged permission decision.
    #[serde(default)]
    pub permission_decision: Option<String>,
    /// The merged rewritten input.
    #[serde(default)]
    pub updated_input: JsValue,
}

/// `Pick<HookOutcome<'PreToolUse'>, 'block' | 'effects'>`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct PreToolUseOutcomeInput {
    /// The block, when a hook held the call back.
    #[serde(default)]
    pub block: Option<OutcomeBlock>,
    /// The merged effects.
    pub effects: OutcomeEffects,
}

/// What the permission check reads (`PreToolUseHookOutcome`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseHookOutcome {
    /// The merged `permissionDecision`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    /// A hook exited with code 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
    /// The answer carries `updatedInput`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_updated_input: Option<bool>,
}

/// What the permission check reads of the `PreToolUse` hooks.
#[must_use]
pub fn pre_tool_use_hook_outcome(outcome: &PreToolUseOutcomeInput) -> PreToolUseHookOutcome {
    let exit_two = outcome
        .block
        .as_ref()
        .is_some_and(|block| block.via.as_deref() == Some("exit2"));
    PreToolUseHookOutcome {
        decision: outcome.effects.permission_decision.clone(),
        blocked: exit_two.then_some(true),
        has_updated_input: (!outcome.effects.updated_input.is_undefined()).then_some(true),
    }
}

crate::registry::exports! {
    "hooks/output/pre-tool-use-outcome" {
        "preToolUseHookOutcome" => ok |outcome: PreToolUseOutcomeInput| pre_tool_use_hook_outcome(&outcome);
    }
}
