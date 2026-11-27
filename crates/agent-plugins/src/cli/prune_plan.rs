//! Port of `src/cli/prune-plan.ts`, switched.
//!
//! `claude plugin prune` as a plan (plugins/cli-reference.md, "plugin prune"): the orphaned
//! auto-installed dependencies at one scope (`user` by default), listed, asked about, or removed by
//! the flags and the terminal. The decision is `planPrune`'s (marketplace); this adds the scope, the
//! result a host prints and the answer to the prompt. "The exit code is `0` whatever you answer at
//! the prompt": a refusal without a terminal exits `0` too (spec/judgments.md).

use serde::{Deserialize, Serialize};

use super::result::{CommandOutput, Extras, command_succeeded};
use super::scope::{ChosenOrError, ScopeChoice, ScopedCommand, choose_scope};
use crate::marketplace::dependency_graph::GraphFacts;
use crate::marketplace::dependency_prune::{PruneOutcome, PruneRun, plan_prune};
use crate::marketplace::install_scopes::InstallScope;

/// The command line of `claude plugin prune`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneArguments {
    /// `--scope`.
    #[serde(default)]
    pub scope: Option<String>,
    /// `--dry-run`.
    #[serde(default)]
    pub dry_run: Option<bool>,
    /// `-y`.
    #[serde(default)]
    pub yes: Option<bool>,
}

/// What the host knows.
#[derive(Clone, Debug, Deserialize)]
pub struct PruneFacts {
    /// The installed plugins and their dependencies.
    pub graph: GraphFacts,
    /// Both stdin and stdout are a terminal.
    pub tty: bool,
}

/// The question the host asks, with what it removes if the answer is yes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PruneQuestion {
    /// The question.
    pub prompt: String,
    /// What a yes removes.
    pub orphans: Vec<String>,
}

/// What the host prints and removes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrunePlan {
    /// The result line.
    pub output: CommandOutput,
    /// The scope pruned.
    pub scope: InstallScope,
    /// The plugin ids the host uninstalls from `scope`.
    pub remove: Vec<String>,
    /// Set when the host asks first; `answerPrune` turns the answer into the final plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<PruneQuestion>,
}

const NOTHING_REMOVED: &str = "Nothing removed";

fn printed(message: &str) -> CommandOutput {
    command_succeeded("prune", message, Extras::default())
}

fn shown(message: &str, scope: InstallScope) -> PrunePlan {
    PrunePlan {
        output: printed(message),
        scope,
        remove: Vec::new(),
        ask: None,
    }
}

/// What each outcome of the decision prints and removes.
fn handle(outcome: PruneOutcome, scope: InstallScope) -> PrunePlan {
    match outcome {
        PruneOutcome::Nothing { message }
        | PruneOutcome::Listed { message, .. }
        | PruneOutcome::Refused { message, .. } => shown(&message, scope),
        PruneOutcome::Removed { orphans, message } => PrunePlan {
            output: printed(&message),
            scope,
            remove: orphans,
            ask: None,
        },
        PruneOutcome::Ask { orphans, prompt } => PrunePlan {
            output: printed(&format!("{}\n{prompt}", orphans.join(", "))),
            scope,
            remove: Vec::new(),
            ask: Some(PruneQuestion { prompt, orphans }),
        },
    }
}

/// What `claude plugin prune` shows and removes, before any answer.
#[must_use]
pub fn plan_prune_command(args: &PruneArguments, facts: &PruneFacts) -> PrunePlan {
    let scope = match choose_scope(ScopedCommand::Prune, args.scope.as_deref()) {
        ChosenOrError::Error(output) => {
            return PrunePlan {
                output,
                scope: InstallScope::User,
                remove: Vec::new(),
                ask: None,
            };
        }
        ChosenOrError::Choice(ScopeChoice::Scope { scope }) => scope,
        // `prune` writes to one scope, `user` when `--scope` is omitted (`DefaultedCommand`).
        ChosenOrError::Choice(ScopeChoice::AutoDetect { .. } | ScopeChoice::AllScopes { .. }) => {
            InstallScope::User
        }
    };
    let run = PruneRun {
        dry_run: args.dry_run,
        yes: args.yes,
        tty: facts.tty,
    };
    handle(plan_prune(&facts.graph, scope, run), scope)
}

/// The plan once the user has answered `Remove? [y/N]`; a plan that asked nothing is returned as it is.
#[must_use]
pub fn answer_prune(plan: PrunePlan, yes: bool) -> PrunePlan {
    let Some(PruneQuestion { orphans, .. }) = plan.ask.clone() else {
        return plan;
    };
    if !yes {
        return shown(NOTHING_REMOVED, plan.scope);
    }
    PrunePlan {
        output: printed(&format!(
            "Removed {} auto-installed plugins: {}",
            orphans.len(),
            orphans.join(", ")
        )),
        scope: plan.scope,
        remove: orphans,
        ask: None,
    }
}

crate::registry::exports! {
    "cli/prune-plan" {
        "planPruneCommand" => ok |args: PruneArguments, facts: PruneFacts| plan_prune_command(&args, &facts);
        "answerPrune" => ok |plan: PrunePlan, yes: bool| answer_prune(plan, yes);
    }
}
