//! Port of `src/marketplace/dependency-prune.ts`, switched.
//!
//! Pruning and uninstalling (plugins/cli-reference.md, `plugin prune`: "Remove auto-installed
//! dependencies that no installed plugin needs anymore. The command never removes a plugin you
//! installed yourself"; `plugin uninstall --prune`: "Also remove auto-installed dependencies that no
//! remaining plugin needs"; plugins/install.md, "Plugins with dependencies": "auto-installed
//! dependencies stay until you run `claude plugin prune`"; plugins/dependencies.md, "Combine
//! constraints from several plugins": "When a user uninstalls the last plugin that constrains a
//! dependency, the dependency ... resumes tracking its marketplace entry"). Pure over `GraphFacts`.

use std::collections::{HashMap, HashSet};

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use super::dependencies::dependency_id;
use super::dependency_graph::{GraphFacts, InstalledRecord, constraints_on};
use super::install_scopes::InstallScope;

fn at_scope(facts: &GraphFacts, scope: InstallScope) -> Vec<&InstalledRecord> {
    facts
        .installed
        .iter()
        .filter(|record| record.scope == scope)
        .collect()
}

/// The ids reachable from `roots` through declared dependencies, among `records` only.
fn reachable(roots: &[&InstalledRecord], records: &[&InstalledRecord]) -> HashSet<String> {
    let by_id: HashMap<&str, &InstalledRecord> = records
        .iter()
        .map(|record| (record.id.as_str(), *record))
        .collect();
    let mut seen: HashSet<String> = HashSet::new();
    let mut pending: Vec<String> = roots.iter().map(|record| record.id.clone()).collect();
    while let Some(id) = pending.pop() {
        if seen.contains(&id) {
            continue;
        }
        let wanted: Vec<String> = by_id
            .get(id.as_str())
            .map(|record| record.dependencies.iter().map(dependency_id).collect())
            .unwrap_or_default();
        seen.insert(id);
        pending.extend(wanted.into_iter().filter(|one| !seen.contains(one)));
    }
    seen
}

/// The auto-installed plugins at `scope` that no remaining plugin needs, transitive orphans
/// included. A plugin the user installed is a root: never removed, and everything it needs, directly
/// or through other dependencies, stays. Judgment: only plugins installed at the same scope count as
/// needing, since a dependency is installed "at the same scope" as its dependent; a user-installed
/// plugin that is disabled still needs its dependencies (it is installed); dependencies that only
/// each other need are orphans.
#[must_use]
pub fn orphaned_auto_installed(facts: &GraphFacts, scope: InstallScope) -> Vec<String> {
    let records = at_scope(facts, scope);
    let roots: Vec<&InstalledRecord> = records
        .iter()
        .copied()
        .filter(|record| record.auto_installed != Some(true))
        .collect();
    let needed = reachable(&roots, &records);
    records
        .iter()
        .filter(|r| r.auto_installed == Some(true) && !needed.contains(&r.id))
        .map(|r| r.id.clone())
        .collect()
}

/// The facts with one install removed.
fn without_install(facts: &GraphFacts, id: &str, scope: InstallScope) -> GraphFacts {
    GraphFacts {
        installed: facts
            .installed
            .iter()
            .filter(|record| record.id != id || record.scope != scope)
            .cloned()
            .collect(),
        ..facts.clone()
    }
}

/// What prune shows and does, by the flags and the terminal (cli-reference.md, `plugin prune`).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneRun {
    /// `--dry-run`.
    #[serde(default)]
    pub dry_run: Option<bool>,
    /// `-y`.
    #[serde(default)]
    pub yes: Option<bool>,
    /// Both stdin and stdout are a terminal.
    pub tty: bool,
}

/// What prune does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PruneOutcome {
    /// Nothing to remove.
    Nothing {
        /// What it says.
        message: String,
    },
    /// The orphans are listed.
    Listed {
        /// The orphans.
        orphans: Vec<String>,
        /// What it says.
        message: String,
    },
    /// The user is asked.
    Ask {
        /// The orphans.
        orphans: Vec<String>,
        /// The question.
        prompt: String,
    },
    /// Nothing is removed without `-y`.
    Refused {
        /// The orphans.
        orphans: Vec<String>,
        /// What it says.
        message: String,
    },
    /// The orphans are removed.
    Removed {
        /// The orphans.
        orphans: Vec<String>,
        /// What it says.
        message: String,
    },
}

/// `claude plugin prune` as a decision (cli-reference.md, `plugin prune`): with none to remove a line
/// that starts `Nothing to prune`; `--dry-run` lists them and ends `(dry run — nothing removed)`;
/// `-y` removes them and prints `Removed N auto-installed plugins: <names>`; an interactive terminal
/// without `-y` asks `Remove? [y/N]`; a pipe without `-y` lists them, prints
/// ``Not a TTY — run `claude plugin prune -y` to remove.`` and removes nothing.
#[must_use]
pub fn plan_prune(facts: &GraphFacts, scope: InstallScope, run: PruneRun) -> PruneOutcome {
    let orphans = orphaned_auto_installed(facts, scope);
    if orphans.is_empty() {
        return PruneOutcome::Nothing {
            message: "Nothing to prune".to_owned(),
        };
    }
    let list = orphans.join(", ");
    if run.dry_run == Some(true) {
        return PruneOutcome::Listed {
            orphans,
            message: format!("{list}\n(dry run — nothing removed)"),
        };
    }
    if run.yes == Some(true) {
        let message = format!("Removed {} auto-installed plugins: {list}", orphans.len());
        return PruneOutcome::Removed { orphans, message };
    }
    if run.tty {
        return PruneOutcome::Ask {
            orphans,
            prompt: "Remove? [y/N]".to_owned(),
        };
    }
    PruneOutcome::Refused {
        orphans,
        message: format!("{list}\nNot a TTY — run `claude plugin prune -y` to remove."),
    }
}

/// What an uninstall removes and frees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UninstallPlan {
    /// The plugin removed from `scope`.
    pub remove: String,
    /// With `--prune`: the auto-installed dependencies no remaining plugin needs, removed with it.
    pub pruned: Vec<String>,
    /// The dependencies the plugin constrained that no other installed plugin constrains any more:
    /// each resumes tracking its marketplace entry on the next update.
    pub released: Vec<String>,
}

/// The dependencies `id` put a range on that nobody else still does once it is gone.
fn released(id: &str, facts: &GraphFacts, after: &GraphFacts) -> Vec<String> {
    let own: IndexSet<String> = facts
        .installed
        .iter()
        .filter(|record| record.id == id)
        .flat_map(|record| record.dependencies.iter())
        .filter(|dep| dep.range.is_some())
        .map(dependency_id)
        .collect();
    own.into_iter()
        .filter(|dependency| {
            !constraints_on(dependency, facts).is_empty()
                && constraints_on(dependency, after).is_empty()
        })
        .collect()
}

/// `claude plugin uninstall <id> [--prune]` as a decision: the plugin goes; its auto-installed
/// dependencies stay unless `prune` is set, then those no remaining plugin needs go too (transitive
/// ones included). Judgment: uninstalling a plugin other plugins depend on is not refused, since the
/// docs state no such check for uninstall (they do for disable); the dependents then fail to load
/// with `is not installed`.
#[must_use]
pub fn plan_uninstall(
    id: &str,
    scope: InstallScope,
    facts: &GraphFacts,
    prune: bool,
) -> UninstallPlan {
    let after_one = without_install(facts, id, scope);
    let pruned = if prune {
        orphaned_auto_installed(&after_one, scope)
    } else {
        Vec::new()
    };
    let after = pruned
        .iter()
        .fold(after_one, |so, one| without_install(&so, one, scope));
    let mut gone: HashSet<&str> = HashSet::from([id]);
    gone.extend(pruned.iter().map(String::as_str));
    let freed: IndexSet<String> = std::iter::once(id)
        .chain(pruned.iter().map(String::as_str))
        .flat_map(|one| released(one, facts, &after))
        .collect();
    UninstallPlan {
        remove: id.to_owned(),
        released: freed
            .into_iter()
            .filter(|one| !gone.contains(one.as_str()))
            .collect(),
        pruned,
    }
}

crate::registry::exports! {
    "marketplace/dependency-prune" {
        "orphanedAutoInstalled" => ok |facts: GraphFacts, scope: InstallScope| orphaned_auto_installed(&facts, scope);
        "planPrune" => ok |facts: GraphFacts, scope: InstallScope, run: PruneRun| plan_prune(&facts, scope, run);
        "planUninstall" => ok |id: String, scope: InstallScope, facts: GraphFacts, prune: bool| plan_uninstall(&id, scope, &facts, prune);
    }
}
