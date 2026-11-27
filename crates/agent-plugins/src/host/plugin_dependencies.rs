//! Port of `src/host/plugin-dependencies.ts`, switched.
//!
//! Which plugins stay off for a dependency (plugins/troubleshooting.md, "Dependency errors": "Claude
//! Code keeps the affected plugin disabled until you resolve it"; plugins/dependencies.md, "Test a
//! plugin and its dependency locally"). The plugins given are what is installed or loaded for the
//! session: one that runs is a copy that meets an entry, one given with `enabled: false` is a
//! disabled copy (`is disabled — enable it or remove the dependency`), one not given is
//! `is not installed`. A `<name>@inline` plugin is the `--plugin-dir` copy (no range checked); any
//! other is installed under its id. Oracle 2.1.322 (spec/judgments.md): with `b@mk` uninstalled or
//! set to `false`, the enabled `a@mk` that depends on it loads nothing, and a plugin that does not
//! load meets no entry, so what needs it stops too (oracle 2.1.319 for `--plugin-dir`).

use indexmap::IndexMap;
use serde::Serialize;

use super::plugin_set::{HostPlugin, PluginRun};
use super::state::HostedPlugin;
use crate::js::JsValue;
use crate::marketplace::dependencies::{
    DependencyIssue, Normalized, names_marketplace, normalize_dependency,
};
use crate::marketplace::dependency_graph::{
    CopyLookup, EnabledPlugins, GraphFacts, InstalledRecord, LocalCopy, check_declared_dependency,
};
use crate::marketplace::install_scopes::InstallScope;

const INLINE: &str = "inline";

/// The marketplace a bare dependency name resolves to: the declaring plugin's own (`inline` for a `--plugin-dir` one).
fn marketplace_of(id: &str) -> Option<&str> {
    id.rfind('@').and_then(|at| id.get(at + 1..))
}

/// The manifest's `dependencies`, or none.
fn declared_of(run: &PluginRun) -> &[JsValue] {
    match run.entry.manifest("dependencies") {
        Some(JsValue::Array(items)) => items,
        _ => &[],
    }
}

fn declares(run: &PluginRun) -> bool {
    run.enabled && !declared_of(run).is_empty()
}

fn version_of(run: &PluginRun) -> Option<String> {
    run.entry
        .manifest("version")
        .and_then(JsValue::as_str)
        .map(str::to_owned)
}

/// The plugins given as the facts a declared dependency is checked against.
fn facts_of(runs: &[PluginRun]) -> GraphFacts {
    let (local, installed): (Vec<&PluginRun>, Vec<&PluginRun>) = runs
        .iter()
        .partition(|run| marketplace_of(&run.entry.id) == Some(INLINE));
    GraphFacts {
        installed: installed
            .iter()
            .map(|run| InstalledRecord {
                id: run.entry.id.clone(),
                scope: InstallScope::User,
                auto_installed: None,
                default_enabled: None,
                version: version_of(run),
                dependencies: Vec::new(),
            })
            .collect(),
        enabled: EnabledPlugins {
            user: Some(
                installed
                    .iter()
                    .map(|run| (run.entry.id.clone(), run.enabled))
                    .collect(),
            ),
            ..EnabledPlugins::default()
        },
        catalog: IndexMap::new(),
        allowlists: IndexMap::new(),
        blocked: None,
        local_copies: Some(
            local
                .iter()
                .map(|run| LocalCopy {
                    name: run.entry.name().to_owned(),
                    enabled: run.enabled,
                    version: version_of(run),
                })
                .collect(),
        ),
    }
}

fn first_issue(run: &PluginRun, facts: &GraphFacts) -> Option<DependencyIssue> {
    for item in declared_of(run) {
        let Normalized::Dependency(dep) =
            normalize_dependency(item, marketplace_of(&run.entry.id), None)
        else {
            continue;
        };
        let lookup = CopyLookup {
            host_version: None,
            names_marketplace: Some(names_marketplace(item)),
        };
        if let Some(issue) = check_declared_dependency(&dep, facts, &lookup) {
            return Some(issue);
        }
    }
    None
}

/// The runs with every plugin whose dependency is not met turned off, and one error per such plugin under its id.
#[must_use]
pub fn with_dependencies(runs: Vec<PluginRun>) -> (Vec<PluginRun>, Vec<DependencyIssue>) {
    let mut current = runs;
    let mut issues = Vec::new();
    if !current.iter().any(declares) {
        return (current, issues);
    }
    loop {
        let facts = facts_of(&current);
        let mut stopped: IndexMap<String, DependencyIssue> = IndexMap::new();
        for run in &current {
            if let Some(issue) = declares(run).then(|| first_issue(run, &facts)).flatten() {
                stopped.insert(run.entry.id.clone(), issue);
            }
        }
        if stopped.is_empty() {
            return (current, issues);
        }
        for (id, mut issue) in stopped.clone() {
            issue.issue.path = id;
            issues.push(issue);
        }
        for run in &mut current {
            if stopped.contains_key(&run.entry.id) {
                run.enabled = false;
            }
        }
    }
}

/// The verdict of `pluginsWithDependencies`.
#[derive(Debug, Serialize)]
pub struct DependencyVerdict {
    /// Each plugin given, with whether it runs.
    pub plugins: Vec<HostedPlugin>,
    /// One error per plugin turned off.
    pub issues: Vec<DependencyIssue>,
}

/// The load-time dependency verdict without a host or a session (docs/API.md, "Host and sessions"):
/// each plugin given with whether it is on, and back whether it runs once every plugin whose
/// dependency is not met is off, transitively, with one error per such plugin under its id (the
/// `errorType` of `claude plugin list --json`). A host gives every installed plugin, a disabled one
/// with `enabled: false`; one not given reads as not installed. The sessions use the same verdict.
#[must_use]
pub fn plugins_with_dependencies(plugins: Vec<HostPlugin>) -> DependencyVerdict {
    let runs = plugins
        .into_iter()
        .map(|entry| PluginRun {
            enabled: entry.enabled.unwrap_or(false),
            entry,
        })
        .collect();
    let (runs, issues) = with_dependencies(runs);
    DependencyVerdict {
        plugins: runs
            .iter()
            .map(|run| HostedPlugin {
                id: run.entry.id.clone(),
                name: run.entry.name().to_owned(),
                enabled: run.enabled,
                core: None,
            })
            .collect(),
        issues,
    }
}

crate::registry::exports! {
    "host/plugin-dependencies" {
        "pluginsWithDependencies" => ok |plugins: Vec<HostPlugin>| plugins_with_dependencies(plugins);
    }
}
