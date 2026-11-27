//! Port of `src/rules/decide/decide.ts`, switched.

use super::context::{DecideContext, ModReach, Pending};
use super::facts::{CallFacts, facts_of};
use super::hooks_and_mods::{
    apply_mod, hook_approves_paired, hook_asks, hook_skips_prompt, hook_vetoes,
};
use super::modes::{ModeStrategy, modes};
use super::rules_stage::rule_behavior;
use super::unanswered::{PromptFacts, settle_prompt};
use crate::registry::Callback;
use crate::rules::types::{Decision, DecisionReason, PermissionBehavior, PreparedCall};

/// Everything a gate reads: the call, the session, what the call is, the mode and what the rules
/// say.
struct Step<'a> {
    call: &'a PreparedCall,
    ctx: &'a DecideContext,
    facts: &'a CallFacts,
    strategy: &'static ModeStrategy,
    verdict: Option<PermissionBehavior>,
}

type Gate = fn(&Step<'_>) -> Option<Pending>;

const fn pending(decision: Decision, reach: ModReach) -> Pending {
    Pending { decision, reach }
}

/// A baseline outcome from the mode is lifted by a mod, except what is put to the host's callback:
/// a prompt (`--permission-prompt-tool` or `canUseTool` "would be asked about") and a
/// classification (permissions.md, "Precedence among rules, hooks, and mods"), which an enforced
/// mod cannot skip.
const fn baseline_reach(decision: Decision) -> ModReach {
    match decision {
        Decision::Allow { .. } | Decision::Deny { .. } => ModReach::Any,
        Decision::Ask { .. } | Decision::Classify { .. } => ModReach::Unbypassable,
    }
}

/// Where a symlink, not the path as asked, leads to a protected path, a review becomes a prompt.
const fn via_link(decision: Decision) -> Decision {
    match decision {
        Decision::Classify { reason, .. } => Decision::Ask { reason },
        Decision::Allow { .. } | Decision::Ask { .. } | Decision::Deny { .. } => decision,
    }
}

const DENIED: Decision = Decision::Deny {
    reason: DecisionReason::DenyRule,
};

/// The tools whose sandboxed command can carry its own allowed domains (sandboxing.md).
const HOSTED_COMMANDS: [&str; 3] = ["Bash", "PowerShell", "Monitor"];

const fn allow(reason: DecisionReason) -> Decision {
    Decision::Allow { reason }
}
const fn deny(reason: DecisionReason) -> Decision {
    Decision::Deny { reason }
}

/// A file the command names or redirects to needs approval, or a redirect writes in the working
/// directories in a mode that does not accept it. A tool-wide `Bash` allow in force lifts it, as
/// `bypassPermissions` does (oracle 2.1.322): a prefix rule allows the command, not the target.
fn redirect_prompts(step: &Step<'_>) -> bool {
    let Step {
        facts,
        ctx,
        strategy,
        ..
    } = step;
    strategy.checks_redirects
        && !(strategy.rules)(&ctx.rules, ctx).iter().any(|rule| {
            rule.behavior == PermissionBehavior::Allow
                && rule.specifier.is_none()
                && rule.value.tool_name == "Bash"
        })
        && (facts.redirects.approval
            || (facts.redirects.in_scope_write && !(strategy.accepts_in_scope_write)(ctx)))
}

/// What the prompt for such a file is: the mode's answer to a call nothing approves (auto
/// classifies).
fn unapproved(facts: &CallFacts) -> CallFacts {
    CallFacts {
        read_only: false,
        reads_files: false,
        edits_in_scope: false,
        shell_edits: false,
        ..facts.clone()
    }
}

/// Each gate is one rule of the docs; the first that answers decides, in the order of the docs.
const GATES: [Gate; 20] = [
    // "The tool never prompts for permission, and PreToolUse hooks don't run for it."
    |step| {
        step.facts
            .exempt
            .then(|| pending(allow(DecisionReason::NoPermissionRequired), ModReach::Any))
    },
    // "the file tools refuse reads there ... in every permission mode, including bypassPermissions"
    |step| {
        step.facts
            .blocked_by
            .map(|reason| pending(deny(reason), ModReach::Never))
    },
    // "A hook that exits with code 2 stops the tool call before permission rules are evaluated."
    |step| hook_vetoes(step.ctx).then(|| pending(deny(DecisionReason::Hook), ModReach::Hook)),
    // "Deny rules block in every mode, including bypassPermissions."
    |step| {
        (step.verdict == Some(PermissionBehavior::Deny))
            .then(|| pending(DENIED, ModReach::Unenforced))
    },
    // "Remove-Item in PowerShell": a system path or a wildcard is denied "in every mode, without asking you".
    |step| {
        step.facts
            .removal_denied
            .then(|| pending(deny(DecisionReason::PowershellRemoval), ModReach::Never))
    },
    // A redirect target an `Edit` or `Read` deny rule covers: "as if Claude wrote or read that file directly".
    |step| {
        step.facts
            .redirects
            .denied
            .then(|| pending(DENIED, ModReach::Unenforced))
    },
    // "Tools matched by an explicit ask rule" are never auto-approved; dontAsk denies them.
    |step| {
        (step.verdict == Some(PermissionBehavior::Ask)).then(|| {
            pending(
                (step.strategy.prompt)(DecisionReason::AskRule),
                ModReach::Unenforced,
            )
        })
    },
    // A hook ask prompts, and auto mode "still shows you those prompts" (with its own paths for a few calls).
    |step| {
        hook_asks(step.ctx).then(|| pending((step.strategy.hook_ask)(step.facts), ModReach::Hook))
    },
    // "Recognized `rm` and `rmdir` critical-path removals, which no allow rule or PreToolUse hook allow approves".
    |step| {
        step.facts
            .critical_removal
            .then(|| pending((step.strategy.critical_removal)(step.ctx), ModReach::Never))
    },
    // "Tools that require user interaction", unless the hook answers with `updatedInput`.
    |step| {
        if !step.facts.interaction {
            return None;
        }
        let approved = hook_approves_paired(step.call, step.ctx);
        let decision = if approved {
            allow(DecisionReason::Hook)
        } else {
            (step.strategy.prompt)(DecisionReason::RequiresUserInteraction)
        };
        Some(pending(decision, ModReach::Never))
    },
    // "Connector tools your organization set to `ask`": no mode, hook or mod approves them.
    |step| {
        step.facts.connector_ask.then(|| {
            pending(
                (step.strategy.prompt)(DecisionReason::OrganizationAsk),
                ModReach::Never,
            )
        })
    },
    // Another organization's public artifact: "Claude Code asks you first, even in auto mode".
    |step| {
        step.facts
            .foreign_artifact
            .then(|| pending((step.strategy.foreign_artifact)(), ModReach::Never))
    },
    // "A read from a network path prompts you even when an allow rule matches."
    |step| {
        step.facts
            .network
            .then(|| pending((step.strategy.network_read)(step.ctx), ModReach::Never))
    },
    // "recognized file-reading Bash commands prompt even in auto mode and `bypassPermissions` mode".
    |step| {
        step.facts.read_block_prompt.then(|| {
            pending(
                (step.strategy.prompt)(DecisionReason::OutsideWorkingDirectory),
                ModReach::Unenforced,
            )
        })
    },
    // "The safety check runs before Claude Code evaluates allow rules from settings."
    |step| {
        if !step.facts.protected_write {
            return None;
        }
        let decision = (step.strategy.protected_write)(step.ctx);
        let decision = if step.facts.protected_via_link {
            via_link(decision)
        } else {
            decision
        };
        Some(pending(decision, ModReach::Never))
    },
    // "A rule such as `Bash(git commit *)` allows the command, not the target."
    |step| {
        redirect_prompts(step).then(|| {
            pending(
                (step.strategy.baseline)(&unapproved(step.facts), step.ctx),
                ModReach::Unenforced,
            )
        })
    },
    // "A command that carries hosts goes to the classifier instead of being approved by a permission rule."
    |step| {
        let carries = step
            .call
            .allowed_domains
            .as_ref()
            .is_some_and(|domains| !domains.is_empty());
        (step.strategy.reviews_command_hosts
            && HOSTED_COMMANDS.contains(&step.call.call.tool_name.as_str())
            && carries)
            .then(|| {
                pending(
                    Decision::Classify {
                        reason: DecisionReason::ModeAuto,
                        then_prompt: None,
                    },
                    ModReach::Unbypassable,
                )
            })
    },
    |step| {
        (step.verdict == Some(PermissionBehavior::Allow))
            .then(|| pending(allow(DecisionReason::AllowRule), ModReach::Any))
    },
    |step| {
        hook_skips_prompt(step.call, step.facts, step.ctx, step.strategy)
            .then(|| pending(allow(DecisionReason::Hook), ModReach::Any))
    },
    |step| {
        step.facts
            .read_only
            .then(|| pending(allow(step.facts.read_reason), ModReach::Any))
    },
];

/// Whether a call may run, asks the user, is denied or goes to the host's classifier
/// (permissions.md, "Manage permissions", "Permission modes" and "Extend permissions with hooks";
/// permission-modes.md). Pure over the call and the session: the host supplies the hook outcome,
/// the mod's answer and the session facts in `ctx`, and acts on the answer.
///
/// The order: a path outside the working directories that the block keys refuse; a hook block;
/// deny rules; ask rules and hook asks; the actions no mode auto-approves (critical-path removals,
/// user interaction, network reads); protected paths; then allow rules, hook allows and read-only
/// calls; then what the mode does with the rest. A mod's answer comes last.
///
/// # Errors
///
/// What a rule's matcher throws.
pub async fn decide(call: &PreparedCall, ctx: &DecideContext) -> Result<Decision, String> {
    let strategy = modes(ctx.mode);
    let facts = facts_of(call, ctx).await?;
    let verdict = rule_behavior(call, &(strategy.rules)(&ctx.rules, ctx), &facts, ctx)?;
    let step = Step {
        call,
        ctx,
        facts: &facts,
        strategy,
        verdict,
    };
    let prompt = PromptFacts {
        cannot_prompt: ctx.cannot_prompt,
        deny_instead_of_prompting: ctx.deny_instead_of_prompting,
    };
    let answer = GATES.iter().find_map(|gate| gate(&step));
    let answer = answer.unwrap_or_else(|| {
        let baseline = (strategy.baseline)(&facts, ctx);
        pending(baseline, baseline_reach(baseline))
    });
    Ok(settle_prompt(
        apply_mod(answer, &facts, ctx, strategy),
        prompt,
    ))
}

/// `decide` over the context and the host's three functions, which cross beside it (a record that
/// holds a function would cross as a port, read member by member).
async fn decide_with(
    call: PreparedCall,
    mut ctx: DecideContext,
    checks: [Option<Callback>; 3],
) -> Result<Decision, String> {
    let [
        requires_permission,
        preapproved_fetch_domain,
        foreign_artifact,
    ] = checks;
    ctx.requires_permission = requires_permission;
    ctx.preapproved_fetch_domain = preapproved_fetch_domain;
    ctx.foreign_artifact = foreign_artifact;
    decide(&call, &ctx).await
}

crate::registry::exports! {
    "rules/decide/decide" {
        "decide" => async fallible |call: PreparedCall, ctx: DecideContext, requires_permission: Option<Callback>, preapproved_fetch_domain: Option<Callback>, foreign_artifact: Option<Callback>|
            decide_with(call, ctx, [requires_permission, preapproved_fetch_domain, foreign_artifact]).await;
    }
}
