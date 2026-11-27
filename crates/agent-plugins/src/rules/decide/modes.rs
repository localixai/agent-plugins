//! Ported from `src/rules/decide/modes.ts`; only the core uses it now, so it has no wrapper.

use std::borrow::Cow;

use super::auto_dropped::is_dropped_in_auto;
use super::behavior::is_allow_rule;
use super::context::{DecideContext, PermissionMode};
use super::facts::CallFacts;
use crate::rules::types::{Decision, DecisionReason, PermissionRule};

const fn allow(reason: DecisionReason) -> Decision {
    Decision::Allow { reason }
}
const fn ask(reason: DecisionReason) -> Decision {
    Decision::Ask { reason }
}
const fn deny(reason: DecisionReason) -> Decision {
    Decision::Deny { reason }
}
const fn classify(reason: DecisionReason) -> Decision {
    Decision::Classify {
        reason,
        then_prompt: None,
    }
}
const fn classify_then_prompt(reason: DecisionReason) -> Decision {
    Decision::Classify {
        reason,
        then_prompt: Some(true),
    }
}

/// The rules in force in a mode: the session's own, or fewer.
pub type RulesInForce =
    for<'a> fn(&'a [PermissionRule], &DecideContext) -> Cow<'a, [PermissionRule]>;

/// What differs between the permission modes (permissions.md, "Permission modes";
/// permission-modes.md), one strategy per mode in a registry typed over `PermissionMode`: a mode
/// added to the SDK fails to compile until it has one. The gates that no mode skips (rules, hooks,
/// mods) are in `decide`.
pub struct ModeStrategy {
    /// What a would-be prompt becomes: a prompt, or in `dontAsk` a denial.
    pub prompt: fn(DecisionReason) -> Decision,
    /// What a PreToolUse hook's `ask` becomes: a prompt, or in `dontAsk` a denial; auto mode has
    /// its own paths.
    pub hook_ask: fn(&CallFacts) -> Decision,
    /// "Protected-path writes", per mode (permission-modes.md, "Protected paths").
    pub protected_write: fn(&DecideContext) -> Decision,
    /// "Critical-path removals in each permission mode".
    pub critical_removal: fn(&DecideContext) -> Decision,
    /// A read from a network path (permissions.md, "Network paths").
    pub network_read: fn(&DecideContext) -> Decision,
    /// A read of another organization's public artifact: always the user's approval, never the
    /// classifier's.
    pub foreign_artifact: fn() -> Decision,
    /// What a call gets when no rule, hook or read-only status settles it.
    pub baseline: fn(&CallFacts, &DecideContext) -> Decision,
    /// Whether a mod's `tool.check` approval can lift an outcome for this call.
    pub mod_may_approve: fn(&CallFacts, &DecideContext) -> bool,
    /// Whether a PreToolUse hook's allow can skip the prompt for this call.
    pub hook_may_allow: fn(&CallFacts, &DecideContext) -> bool,
    /// Whether a redirect target outside what the rules and directories cover prompts
    /// (`bypassPermissions` checks none).
    pub checks_redirects: bool,
    /// Whether a command carrying per-command allowed domains goes to the classifier whatever the
    /// allow rules say.
    pub reviews_command_hosts: bool,
    /// Whether a redirect that writes in the working directories passes as an edit there, not a
    /// prompt.
    pub accepts_in_scope_write: fn(&DecideContext) -> bool,
    /// The rules in force in this mode.
    pub rules: RulesInForce,
}

/// Manual mode: reads in scope only; everything else asks. The other modes change what they must.
const PROMPTING: ModeStrategy = ModeStrategy {
    prompt: ask,
    hook_ask: |_| ask(DecisionReason::Hook),
    protected_write: |_| ask(DecisionReason::ProtectedPath),
    critical_removal: |_| ask(DecisionReason::CriticalPath),
    network_read: |_| ask(DecisionReason::NetworkPath),
    foreign_artifact: || ask(DecisionReason::ForeignArtifact),
    baseline: |_, _| ask(DecisionReason::DefaultPrompt),
    mod_may_approve: |_, _| true,
    hook_may_allow: |_, _| true,
    checks_redirects: true,
    reviews_command_hosts: false,
    accepts_in_scope_write: |_| false,
    rules: |rules, _| Cow::Borrowed(rules),
};

/// A hook's `ask` in auto mode "normally sends the tool call to the permission prompt, and the
/// classifier doesn't decide it". Two paths differ (hooks.md, "PreToolUse `ask` decisions in auto
/// mode"): a call Claude Code's own checks flag too, a protected-path write or a command that asks
/// to run outside the sandbox, which the classifier reviews first ("can deny one of these calls
/// before any prompt appears. If it approves, the call normally still goes to the prompt"); and
/// `SubagentHandback`, which no prompt guards ("the call runs even if the classifier objects").
const fn auto_hook_ask(facts: &CallFacts) -> Decision {
    if facts.handback {
        return allow(DecisionReason::NoPermissionRequired);
    }
    if facts.protected_write || facts.unsandboxed {
        classify_then_prompt(DecisionReason::Hook)
    } else {
        ask(DecisionReason::Hook)
    }
}

/// "Interactive terminal sessions with bypass permissions available": plan mode's blocks are not
/// enforced.
const fn bypass_in_plan(ctx: &DecideContext) -> bool {
    ctx.bypass_available && ctx.interactive
}

/// Auto mode available with `useAutoModeDuringPlan` on, "which it is by default".
fn auto_in_plan(ctx: &DecideContext) -> bool {
    ctx.auto_available && ctx.use_auto_mode_during_plan != Some(false)
}

/// In auto mode a removal asks at a terminal and is denied elsewhere. With
/// `permissions.denyInsteadOfPrompting` there is no prompt to ask at: the removal is "denied
/// before any hook is asked" (settings-reference.md, "What else `denyInsteadOfPrompting` stops").
fn auto_removal(ctx: &DecideContext) -> Decision {
    if ctx.interactive && ctx.deny_instead_of_prompting != Some(true) {
        ask(DecisionReason::CriticalPath)
    } else {
        deny(DecisionReason::CriticalPath)
    }
}

/// A protected-path write goes to the classifier, except "in a session started with
/// `--restricted` ... the classifier can't approve protected-path writes": then the user is asked.
fn classified_protected_write(ctx: &DecideContext) -> Decision {
    if ctx.options().restricted == Some(true) {
        ask(DecisionReason::ProtectedPath)
    } else {
        classify(DecisionReason::ProtectedPath)
    }
}

fn plan_protected_write(ctx: &DecideContext) -> Decision {
    if bypass_in_plan(ctx) {
        return allow(DecisionReason::ModePlan);
    }
    if auto_in_plan(ctx) {
        classified_protected_write(ctx)
    } else {
        ask(DecisionReason::ProtectedPath)
    }
}

fn plan_removal(ctx: &DecideContext) -> Decision {
    if auto_in_plan(ctx) && !bypass_in_plan(ctx) {
        auto_removal(ctx)
    } else {
        ask(DecisionReason::CriticalPath)
    }
}

fn plan_baseline(facts: &CallFacts, ctx: &DecideContext) -> Decision {
    if bypass_in_plan(ctx) {
        return allow(DecisionReason::ModePlan);
    }
    if facts.shell && auto_in_plan(ctx) {
        classify(DecisionReason::ModePlan)
    } else {
        ask(DecisionReason::ModePlan)
    }
}

/// Plan mode: "the mod can't approve a call that isn't read-only", nor can a hook's allow.
const fn plan_lifts(facts: &CallFacts, ctx: &DecideContext) -> bool {
    facts.read_only || bypass_in_plan(ctx)
}

const ACCEPT_EDITS: ModeStrategy = ModeStrategy {
    accepts_in_scope_write: |_| true,
    baseline: |facts, _| {
        if facts.edits_in_scope || facts.shell_edits {
            allow(DecisionReason::AcceptEdits)
        } else {
            ask(DecisionReason::DefaultPrompt)
        }
    },
    ..PROMPTING
};

const PLAN: ModeStrategy = ModeStrategy {
    protected_write: plan_protected_write,
    critical_removal: plan_removal,
    network_read: |ctx| {
        if bypass_in_plan(ctx) {
            allow(DecisionReason::ModePlan)
        } else {
            ask(DecisionReason::NetworkPath)
        }
    },
    baseline: plan_baseline,
    // With auto mode in plan the write goes to the classifier, as in auto mode.
    accepts_in_scope_write: |ctx| auto_in_plan(ctx) || bypass_in_plan(ctx),
    mod_may_approve: plan_lifts,
    hook_may_allow: plan_lifts,
    ..PROMPTING
};

const AUTO: ModeStrategy = ModeStrategy {
    hook_ask: auto_hook_ask,
    // "a rule approves the command, not its hosts" (permission-modes.md, "How the classifier evaluates actions").
    reviews_command_hosts: true,
    // The classifier judges the command: the write in the working directories is no prompt of its own.
    accepts_in_scope_write: |_| true,
    protected_write: classified_protected_write,
    critical_removal: auto_removal,
    baseline: |facts, _| {
        if facts.reads_files || facts.edits_in_scope {
            return allow(DecisionReason::ModeAuto);
        }
        if facts.escapes_via_link {
            ask(DecisionReason::DefaultPrompt)
        } else {
            classify(DecisionReason::ModeAuto)
        }
    },
    rules: |rules, ctx| {
        let all_shell = ctx.classify_all_shell == Some(true);
        Cow::Owned(
            rules
                .iter()
                .filter(|rule| !is_dropped_in_auto(rule, all_shell))
                .cloned()
                .collect(),
        )
    },
    ..PROMPTING
};

const DONT_ASK: ModeStrategy = ModeStrategy {
    prompt: deny,
    hook_ask: |_| deny(DecisionReason::Hook),
    protected_write: |_| deny(DecisionReason::ProtectedPath),
    critical_removal: |_| deny(DecisionReason::CriticalPath),
    network_read: |_| deny(DecisionReason::NetworkPath),
    foreign_artifact: || deny(DecisionReason::ForeignArtifact),
    baseline: |_, _| deny(DecisionReason::ModeDontask),
    ..PROMPTING
};

const BYPASS_PERMISSIONS: ModeStrategy = ModeStrategy {
    protected_write: |_| allow(DecisionReason::ModeBypass),
    critical_removal: |ctx| {
        if ctx.critical_path_prompt_off == Some(true) {
            allow(DecisionReason::ModeBypass)
        } else {
            ask(DecisionReason::CriticalPath)
        }
    },
    network_read: |_| allow(DecisionReason::ModeBypass),
    // "Where Claude Code can't ask you, such as in `bypassPermissions` mode, Claude can't read the artifact."
    foreign_artifact: || deny(DecisionReason::ForeignArtifact),
    baseline: |_, _| allow(DecisionReason::ModeBypass),
    checks_redirects: false,
    // "Allow rules have no effect in `bypassPermissions`": everything runs without them.
    rules: |rules, _| {
        Cow::Owned(
            rules
                .iter()
                .filter(|rule| !is_allow_rule(rule))
                .cloned()
                .collect(),
        )
    },
    ..PROMPTING
};

/// `MODES[mode]`.
#[must_use]
pub const fn modes(mode: PermissionMode) -> &'static ModeStrategy {
    match mode {
        PermissionMode::Default => &PROMPTING,
        PermissionMode::AcceptEdits => &ACCEPT_EDITS,
        PermissionMode::Plan => &PLAN,
        PermissionMode::Auto => &AUTO,
        PermissionMode::DontAsk => &DONT_ASK,
        PermissionMode::BypassPermissions => &BYPASS_PERMISSIONS,
    }
}
