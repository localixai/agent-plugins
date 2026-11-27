//! Port of `src/marketplace/uninstall-records.ts`, switched.
//!
//! Which install records one `claude plugin uninstall` removes (plugins/troubleshooting.md, "A
//! plugin stays installed after `plugin uninstall` on Windows"): at project or local scope a
//! record belongs to a project folder, and "`installed_plugins.json` held two install records of
//! the plugin for the project folder, each spelling the folder's path differently, and one
//! uninstall removes one of them". The record under the spelling the command was run with goes
//! first; "the second run finds no record under its own spelling of the path, so it removes the
//! one under the other spelling" (from v2.1.295). Pure over the records; the host rewrites the
//! file.
//!
//! Judgments (spec/judgments.md): "spelling" is the text of `projectPath` as written; the other
//! spelling is found by the platform's own comparison of paths (on Windows, capitals and the two
//! separators do not tell folders apart; elsewhere nothing but the text does); a project or local
//! uninstall that names no folder removes nothing and says so, since the scope is one folder's and
//! removing every project's record would uninstall the plugin everywhere; when no record matches
//! the command says `Plugin "<name>" is not installed in <scope> scope`; an uninstall marks the
//! version folders it leaves behind, inside the cache only (`dirsToOrphan`).
//!
//! The TypeScript tells the removed records from the kept ones by identity
//! (`!removed.includes(one)`); `removed` is always a selection of `records`, so here each record
//! is told by its place in `records`, which is the same partition.

use serde::{Deserialize, Serialize};

use super::command_output::same_path;
use super::install_scopes::InstallScope;
use super::installed_plugins::InstallRecord;
use super::orphan_cleanup::dirs_to_orphan;
use crate::format::issues::{PluginIssue, error};
use crate::format::path_text::PathPlatform;
use crate::registry::Port;

/// What one uninstall names.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallSelector {
    /// What the command names, for its messages: `<name>` or `<name>@<marketplace>`.
    pub plugin: String,
    /// The host's `StoreLayout`.
    pub layout: Port,
    /// The scope.
    pub scope: InstallScope,
    /// The project folder the command ran in, as the command spells it; `project` and `local`
    /// scope only.
    #[serde(default)]
    pub project_path: Option<String>,
    /// Whose path rules apply.
    pub platform: PathPlatform,
}

/// What one uninstall removes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UninstallSelection {
    /// The records it removes.
    pub removed: Vec<InstallRecord>,
    /// The records it keeps.
    pub kept: Vec<InstallRecord>,
    /// The version directories the uninstall leaves behind: write `.orphaned_at` into each.
    pub orphaned: Vec<String>,
    /// What is wrong.
    pub issues: Vec<PluginIssue>,
}

const PER_PROJECT: [InstallScope; 2] = [InstallScope::Project, InstallScope::Local];

/// The places in `records` of the records of the scope that belong to the folder the command
/// named, exact spelling first.
fn in_scope(records: &[InstallRecord], selector: &UninstallSelector) -> Vec<usize> {
    let of_scope: Vec<usize> = (0..records.len())
        .filter(|at| {
            records
                .get(*at)
                .is_some_and(|one| one.scope == selector.scope)
        })
        .collect();
    if !PER_PROJECT.contains(&selector.scope) {
        return of_scope;
    }
    let Some(wanted) = selector.project_path.as_deref() else {
        return Vec::new();
    };
    let project_of = |at: &usize| records.get(*at).and_then(|one| one.project_path.as_deref());
    let exact: Vec<usize> = of_scope
        .iter()
        .copied()
        .filter(|at| project_of(at) == Some(wanted))
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    of_scope
        .into_iter()
        .filter(|at| project_of(at).is_some_and(|path| same_path(path, wanted, selector.platform)))
        .collect()
}

/// The refusal for a project or local scope without the folder the command ran in.
fn needs_folder(scope: InstallScope) -> String {
    format!(
        "The {} scope belongs to one project folder: name the folder the command runs in",
        scope.as_str()
    )
}

/// What the command says when no record is left to remove: `Plugin "<name>" is not installed in
/// <scope> scope`.
fn not_installed_in_scope(name: &str, scope: InstallScope) -> String {
    format!(
        "Plugin \"{name}\" is not installed in {} scope",
        scope.as_str()
    )
}

fn issues_of(selector: &UninstallSelector, removed: &[InstallRecord]) -> Vec<PluginIssue> {
    if !removed.is_empty() {
        return Vec::new();
    }
    let needs = PER_PROJECT.contains(&selector.scope) && selector.project_path.is_none();
    vec![error(
        "plugin",
        &if needs {
            needs_folder(selector.scope)
        } else {
            not_installed_in_scope(&selector.plugin, selector.scope)
        },
    )]
}

/// Which records one uninstall removes, which it keeps, and the version folders it leaves behind.
pub async fn select_uninstall(
    records: &[InstallRecord],
    selector: &UninstallSelector,
) -> UninstallSelection {
    let chosen = in_scope(records, selector);
    let mut removed = Vec::new();
    for at in &chosen {
        removed.extend(records.get(*at).cloned());
    }
    let kept: Vec<InstallRecord> = records
        .iter()
        .enumerate()
        .filter(|(at, _)| !chosen.contains(at))
        .map(|(_, one)| one.clone())
        .collect();
    let orphaned = dirs_to_orphan(&removed, &kept, &selector.layout).await;
    let issues = issues_of(selector, &removed);
    UninstallSelection {
        removed,
        kept,
        orphaned,
        issues,
    }
}

crate::registry::exports! {
    "marketplace/uninstall-records" {
        "selectUninstall" => async ok |records: Vec<InstallRecord>, selector: UninstallSelector|
            select_uninstall(&records, &selector).await;
    }
}
