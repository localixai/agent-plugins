//! Ported from `src/rules/decide/hooks-and-mods.ts`; only the core uses it now, so it has no wrapper.
//!
//! How PreToolUse hooks and mods meet the rules and the mode (permissions.md, "Extend permissions
//! with hooks"; hooks.md, "PreToolUse decision control").

use super::context::{DecideContext, HookPermissionDecision, ModOutcome, ModReach, Pending};
use super::facts::CallFacts;
use super::modes::ModeStrategy;
use crate::rules::types::{Decision, DecisionReason, PreparedCall};

/// What one answer of a PreToolUse hook does to the permission check.
#[derive(Clone, Copy)]
struct HookAnswer {
    vetoes: bool,
    asks: bool,
    allows: bool,
}

const NO_ANSWER: HookAnswer = HookAnswer {
    vetoes: false,
    asks: false,
    allows: false,
};

/// Typed over the SDK's union: a decision added to it fails to compile until it has an entry.
/// `defer` is no answer here.
const fn hook_answers(decision: HookPermissionDecision) -> HookAnswer {
    match decision {
        HookPermissionDecision::Allow => HookAnswer {
            vetoes: false,
            asks: false,
            allows: true,
        },
        HookPermissionDecision::Deny => HookAnswer {
            vetoes: true,
            asks: false,
            allows: false,
        },
        HookPermissionDecision::Ask => HookAnswer {
            vetoes: false,
            asks: true,
            allows: false,
        },
        HookPermissionDecision::Defer => NO_ANSWER,
    }
}

fn answer_of(ctx: &DecideContext) -> HookAnswer {
    ctx.hook
        .and_then(|hook| hook.decision)
        .map_or(NO_ANSWER, hook_answers)
}

/// A hook that "exits with code 2" blocks, and a `deny` is a block; JSON on an exit-2 hook cannot
/// override it.
pub fn hook_vetoes(ctx: &DecideContext) -> bool {
    ctx.hook.and_then(|hook| hook.blocked) == Some(true) || answer_of(ctx).vetoes
}

/// A hook `ask` "prompts the user to confirm". `defer` is not an answer here.
pub fn hook_asks(ctx: &DecideContext) -> bool {
    answer_of(ctx).asks
}

/// Tools whose allow needs the hook's own answer: "`AskUserQuestion` and `ExitPlanMode` need
/// `updatedInput` paired with `\"allow\"`". For them an allow without `updatedInput` does not skip
/// the prompt, and with it the tool runs in any mode (plan mode included).
const PAIRED_TOOLS: [&str; 2] = ["AskUserQuestion", "ExitPlanMode"];

fn hook_allowed(ctx: &DecideContext) -> bool {
    answer_of(ctx).allows
}

/// Whether the hook's allow settles a call of a paired tool.
pub fn hook_approves_paired(call: &PreparedCall, ctx: &DecideContext) -> bool {
    PAIRED_TOOLS.contains(&call.call.tool_name.as_str())
        && hook_allowed(ctx)
        && ctx.hook.and_then(|hook| hook.has_updated_input) == Some(true)
}

/// Whether the hook's allow skips the prompt a call would get: not for a paired tool, not where
/// the mode ignores it.
pub fn hook_skips_prompt(
    call: &PreparedCall,
    facts: &CallFacts,
    ctx: &DecideContext,
    strategy: &ModeStrategy,
) -> bool {
    if PAIRED_TOOLS.contains(&call.call.tool_name.as_str()) {
        return hook_approves_paired(call, ctx);
    }
    hook_allowed(ctx) && (strategy.hook_may_allow)(facts, ctx)
}

/// "With neither managed settings nor a Team or Enterprise /login ... the mod can approve a call
/// that a deny rule refuses or a PreToolUse hook blocked", and is not prompted for an ask rule or
/// a hook ask; under policy, `allowModsToOverrideDenyRules` lets it past the same outcomes
/// (J14.14).
fn unenforced(r#mod: ModOutcome) -> bool {
    !r#mod.policy_enforced || r#mod.overrides_deny_rules == Some(true)
}

/// How far a mod's approval reaches, as a test on the mod and the session.
fn reach(reach: ModReach, r#mod: ModOutcome, ctx: &DecideContext) -> bool {
    match reach {
        ModReach::Any => true,
        // "except where the mod can't bypass your permission prompt tool or Agent SDK permission callback"
        ModReach::Unbypassable => !(r#mod.policy_enforced && ctx.permission_callback == Some(true)),
        ModReach::Unenforced => unenforced(r#mod),
        // A managed hook's block or prompt stays whatever the option says (Agent Plugins Extended J14.14).
        ModReach::Hook => ctx.hook.and_then(|hook| hook.managed) != Some(true) && unenforced(r#mod),
        ModReach::Never => false,
    }
}

const MOD_DENIAL: Decision = Decision::Deny {
    reason: DecisionReason::Mod,
};
const MOD_APPROVAL: Decision = Decision::Allow {
    reason: DecisionReason::Mod,
};

/// A mod's refusal stops a call that was going to run; a denial already made stands.
const fn blocked(decision: Decision) -> Decision {
    match decision {
        Decision::Allow { .. } | Decision::Ask { .. } | Decision::Classify { .. } => MOD_DENIAL,
        Decision::Deny { .. } => decision,
    }
}

/// A mod's approval lifts what would have stopped or prompted a call; an allow already made
/// stands.
const fn approved(decision: Decision) -> Decision {
    match decision {
        Decision::Allow { .. } => decision,
        Decision::Ask { .. } | Decision::Classify { .. } | Decision::Deny { .. } => MOD_APPROVAL,
    }
}

/// The decision once a mod that handles `tool.check` has answered: it can block a call, and its
/// approval lifts an outcome as far as `Pending.reach` and the mode allow.
pub fn apply_mod(
    pending: Pending,
    facts: &CallFacts,
    ctx: &DecideContext,
    strategy: &ModeStrategy,
) -> Decision {
    let Pending {
        decision,
        reach: how_far,
    } = pending;
    let Some(r#mod) = ctx.mod_outcome else {
        return decision;
    };
    if r#mod.blocked == Some(true) {
        return blocked(decision);
    }
    let lifts =
        r#mod.approved && (strategy.mod_may_approve)(facts, ctx) && reach(how_far, r#mod, ctx);
    if lifts { approved(decision) } else { decision }
}
