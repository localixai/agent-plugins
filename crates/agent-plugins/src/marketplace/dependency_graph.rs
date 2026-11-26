//! Port of `src/marketplace/dependency-graph.ts`, switched.
//!
//! The facts the dependency graph reads, as plain data (plugins/dependencies.md; plugins/install.md,
//! "Plugins with dependencies"; plugins/cli-reference.md, `plugin enable`, `plugin disable`,
//! `plugin prune`). The kit has no disk and no settings reader: a host (or the settings and updates
//! modules) reads `installed_plugins.json`, the `enabledPlugins` of each scope and the marketplace
//! catalog, and passes what it found. Every function of the graph is pure over these facts.
//!
//! A catalog plugin's `source` (`format/marketplace/types.ts`'s `PluginSource`) belongs to a module
//! not ported yet, and nothing here reads it, so it crosses as the value it is (`JsValue`).

use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::marketplace::cross_marketplace::DeclaredIn;
use crate::marketplace::dependencies::{
    DependencyIssue, InstalledDependency, check_installed_dependency, dependency_id,
};
use crate::marketplace::install_scopes::{InstallScope, SCOPES_BY_PRECEDENCE};
use crate::marketplace::semver::satisfies_range;
use crate::marketplace::types::NormalizedDependency;
use crate::registry::Set;

/// A dependency as a manifest or a marketplace entry declares it: where it was declared decides
/// what a cross-marketplace refusal does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredDependency {
    /// The dependency.
    pub dep: NormalizedDependency,
    /// `entry` or `plugin.json`.
    pub declared_in: DeclaredIn,
    /// The item names a marketplace itself (`namesMarketplace(item)`); absent means it does not,
    /// which a `--plugin-dir` copy meets on every Claude Code version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub names_marketplace: Option<bool>,
}

/// One install of a plugin at one scope (`installed_plugins.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledRecord {
    /// `name@marketplace`.
    pub id: String,
    /// The scope it is installed in.
    pub scope: InstallScope,
    /// Installed as another plugin's dependency, not by the user: only such a plugin is pruned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_installed: Option<bool>,
    /// The version constraints are checked against: the tag's for a tag-resolved copy, else the
    /// `plugin.json` version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The plugin's `defaultEnabled` (the entry's over `plugin.json`'s, `mergeEntryWithManifest`);
    /// absent is `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_enabled: Option<bool>,
    /// The plugin's own dependencies, normalized.
    pub dependencies: Vec<NormalizedDependency>,
}

/// `enabledPlugins` of each scope: the value written for a plugin id, if any.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnabledPlugins {
    /// The user scope's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<IndexMap<String, bool>>,
    /// The project scope's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<IndexMap<String, bool>>,
    /// The local scope's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<IndexMap<String, bool>>,
    /// The managed scope's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed: Option<IndexMap<String, bool>>,
}

impl EnabledPlugins {
    /// `enabled[scope]?.[id]`.
    pub fn value(&self, scope: InstallScope, id: &str) -> Option<bool> {
        let written = match scope {
            InstallScope::User => &self.user,
            InstallScope::Project => &self.project,
            InstallScope::Local => &self.local,
            InstallScope::Managed => &self.managed,
        };
        written.as_ref()?.get(id).copied()
    }
}

/// A plugin loaded for one session with `--plugin-dir`: it is `<name>@inline`, whatever its
/// manifest says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalCopy {
    /// Its name.
    pub name: String,
    /// Whether it is enabled.
    pub enabled: bool,
    /// Its version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// What the marketplace catalog says about a plugin the dependency graph may have to install.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPlugin {
    /// Its name.
    pub name: String,
    /// Its marketplace.
    pub marketplace: String,
    /// Where it is installed from (a `PluginSource`).
    pub source: JsValue,
    /// The dependencies of its marketplace entry and of its `plugin.json`, each with where it was
    /// declared (`catalogDependencies`).
    pub dependencies: Vec<DeclaredDependency>,
    /// Its `plugin.json` is not read yet, so `dependencies` holds the entry's only: before
    /// install, Claude Code reads `plugin.json` only for a relative-path source
    /// (marketplace-reference.md, "Display fields"). An install plan that reaches it lists it in
    /// `needs`; the host fetches it, fills `dependencies` (and `currentCopyVersion`) from its
    /// `plugin.json`, drops the flag and plans again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_unread: Option<bool>,
    /// The tags of the repository that hosts the plugin; none for a source with no tags or a
    /// folder that is not a git repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// The commit each tag names (by tag name), for the 12-character suffix `claude plugin list`
    /// shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_commits: Option<IndexMap<String, String>>,
    /// The `version` in the `plugin.json` of the marketplace's current copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_copy_version: Option<String>,
    /// The entry sets a `headersHelper`: Claude Code never installs it as a dependency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers_helper: Option<String>,
    /// The plugin's `defaultEnabled` (the entry's over `plugin.json`'s); absent is `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_enabled: Option<bool>,
}

/// What the graph reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphFacts {
    /// Every install record.
    pub installed: Vec<InstalledRecord>,
    /// `enabledPlugins` of each scope.
    pub enabled: EnabledPlugins,
    /// The catalog by plugin id (`name@marketplace`).
    pub catalog: IndexMap<String, CatalogPlugin>,
    /// `allowCrossMarketplaceDependenciesOn` of each marketplace, by its name; only the root's is
    /// read.
    pub allowlists: IndexMap<String, Vec<String>>,
    /// Plugin ids the organization's plugin policy blocks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<Set<String>>,
    /// The `--plugin-dir` copies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_copies: Option<Vec<LocalCopy>>,
}

/// Every install record of `id`.
#[must_use]
pub fn records_of<'f>(id: &str, facts: &'f GraphFacts) -> Vec<&'f InstalledRecord> {
    facts
        .installed
        .iter()
        .filter(|record| record.id == id)
        .collect()
}

/// The install record of `id` at `scope`.
#[must_use]
pub fn record_at<'f>(
    id: &str,
    scope: InstallScope,
    facts: &'f GraphFacts,
) -> Option<&'f InstalledRecord> {
    facts
        .installed
        .iter()
        .find(|record| record.id == id && record.scope == scope)
}

/// The highest-precedence scope whose `enabledPlugins` lists `id`, if any does.
#[must_use]
pub fn winning_scope(id: &str, facts: &GraphFacts) -> Option<InstallScope> {
    SCOPES_BY_PRECEDENCE
        .into_iter()
        .find(|&scope| facts.enabled.value(scope, id).is_some())
}

/// The value the winning scope writes for `id`, if any scope writes one.
#[must_use]
pub fn effective_enabled(id: &str, facts: &GraphFacts) -> Option<bool> {
    let writer = winning_scope(id, facts)?;
    facts.enabled.value(writer, id)
}

/// Whether an installed plugin is on: the value the winning scope writes, else its
/// `defaultEnabled` ("Default `true`. Whether the plugin starts enabled when the user hasn't set it
/// in `enabledPlugins`", marketplace-reference.md). A plugin with no install record is not on.
#[must_use]
pub fn is_enabled(id: &str, facts: &GraphFacts) -> bool {
    let Some(record) = records_of(id, facts).first().copied() else {
        return false;
    };
    effective_enabled(id, facts)
        .or(record.default_enabled)
        .unwrap_or(true)
}

/// The plugin ids installed, once each however many scopes hold them.
fn installed_ids(facts: &GraphFacts) -> Vec<&str> {
    let ids: IndexSet<&str> = facts
        .installed
        .iter()
        .map(|record| record.id.as_str())
        .collect();
    ids.into_iter().collect()
}

/// The dependencies a plugin declares, from any of its installed records (the manifest does not
/// differ per scope).
#[must_use]
pub fn dependencies_of<'f>(id: &str, facts: &'f GraphFacts) -> &'f [NormalizedDependency] {
    records_of(id, facts)
        .first()
        .map_or(&[], |record| record.dependencies.as_slice())
}

/// An installed plugin that constrains `dependency` with a range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constraint {
    /// The range.
    pub range: String,
    /// The id of the constraining plugin.
    pub by: String,
}

/// The ranges installed plugins put on a dependency (plugins/dependencies.md, "Combine constraints
/// from several plugins"), once per constraining plugin. Judgment: every installed plugin counts,
/// whatever its scope or whether it is enabled; the docs say "installed plugins".
#[must_use]
pub fn constraints_on(dependency: &str, facts: &GraphFacts) -> Vec<Constraint> {
    installed_ids(facts)
        .into_iter()
        .flat_map(|by| {
            dependencies_of(by, facts).iter().filter_map(move |dep| {
                let range = dep
                    .range
                    .as_ref()
                    .filter(|_| dependency_id(dep) == dependency)?;
                Some(Constraint {
                    range: range.clone(),
                    by: by.to_owned(),
                })
            })
        })
        .collect()
}

/// What "installed on a host that loads the copy" means for the `--plugin-dir` copy of a
/// dependency.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyLookup {
    /// The Claude Code version being emulated; an entry that names a marketplace matches a local
    /// copy from 2.1.242. Absent means a current one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_version: Option<String>,
    /// The entry names a marketplace itself (`namesMarketplace(item)`), not merely resolves to its
    /// plugin's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub names_marketplace: Option<bool>,
}

const NAMED_ENTRY_MATCH: &str = ">=2.1.242";

fn from_record(id: &str, facts: &GraphFacts) -> Option<InstalledDependency> {
    let records = records_of(id, facts);
    let record = records
        .iter()
        .find(|one| facts.enabled.value(one.scope, id) == Some(true))
        .or_else(|| records.first())?;
    Some(InstalledDependency {
        version: record.version.clone(),
        enabled: is_enabled(id, facts),
        local: None,
    })
}

fn local_matches(lookup: &CopyLookup) -> bool {
    lookup.names_marketplace != Some(true)
        || lookup
            .host_version
            .as_deref()
            .is_none_or(|version| satisfies_range(version, NAMED_ENTRY_MATCH))
}

/// The `--plugin-dir` copy of `dep`'s name, when it meets the entry on the host `lookup` names.
#[must_use]
pub fn local_copy_of(
    dep: &NormalizedDependency,
    facts: &GraphFacts,
    lookup: &CopyLookup,
) -> Option<InstalledDependency> {
    let local = facts
        .local_copies
        .as_ref()?
        .iter()
        .find(|copy| copy.name == dep.name)?;
    if !local_matches(lookup) {
        return None;
    }
    Some(InstalledDependency {
        version: local.version.clone(),
        enabled: local.enabled,
        local: Some(true),
    })
}

/// The copy that meets a dependency entry (plugins/dependencies.md, "Test a plugin and its
/// dependency locally"): the one installed from the marketplace, else the `--plugin-dir` copy of
/// the same name. The local copy meets an entry with no marketplace and, from Claude Code 2.1.242,
/// one that names a marketplace; no range is checked against it. Judgment: an installed copy wins
/// over a local one, as "until you install the dependency from its marketplace" puts the local
/// copy first only until then.
#[must_use]
pub fn find_dependency_copy(
    dep: &NormalizedDependency,
    facts: &GraphFacts,
    lookup: &CopyLookup,
) -> Option<InstalledDependency> {
    from_record(&dependency_id(dep), facts).or_else(|| local_copy_of(dep, facts, lookup))
}

/// The load-time check of one declared dependency against what is installed or loaded locally.
#[must_use]
pub fn check_declared_dependency(
    dep: &NormalizedDependency,
    facts: &GraphFacts,
    lookup: &CopyLookup,
) -> Option<DependencyIssue> {
    check_installed_dependency(dep, find_dependency_copy(dep, facts, lookup).as_ref())
}

/// What `claude plugin disable` does with a plugin other plugins still need.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum DisableDecision {
    /// `{ allowed: true }`.
    Allowed {
        /// Always true.
        allowed: bool,
    },
    /// `{ allowed: false, dependents, chain, command, message }`.
    Refused {
        /// Always false.
        allowed: bool,
        /// The enabled plugins that declare the target directly.
        dependents: Vec<String>,
        /// Every plugin to disable, in the order that works: dependents of dependents first, the
        /// target last.
        chain: Vec<String>,
        /// The chained command that disables them in that order.
        command: String,
        /// The words.
        message: String,
    },
}

/// The enabled plugins that declare `id` as a dependency.
#[must_use]
pub fn dependents_of(id: &str, facts: &GraphFacts) -> Vec<String> {
    installed_ids(facts)
        .into_iter()
        .filter(|&other| {
            other != id
                && is_enabled(other, facts)
                && dependencies_of(other, facts)
                    .iter()
                    .any(|dep| dependency_id(dep) == id)
        })
        .map(str::to_owned)
        .collect()
}

/// Post-order over dependents, so a plugin comes after everything that needs it; cycle-safe.
fn disable_order(id: &str, facts: &GraphFacts, seen: &mut HashSet<String>) -> Vec<String> {
    if !seen.insert(id.to_owned()) {
        return Vec::new();
    }
    let mut order: Vec<String> = Vec::new();
    for one in dependents_of(id, facts) {
        order.extend(disable_order(&one, facts, seen));
    }
    order.push(id.to_owned());
    order
}

/// Whether a plugin may be disabled (plugins/install.md, "Plugins with dependencies": "when another
/// enabled plugin still needs the one you named, Claude Code refuses and prints a chained command
/// that disables both in the right order"; cli-reference.md, `plugin disable`: "names the
/// dependents to disable first"). The refusal text is a judgment; the docs quote none.
#[must_use]
pub fn disable_decision(id: &str, facts: &GraphFacts) -> DisableDecision {
    let dependents = dependents_of(id, facts);
    if dependents.is_empty() {
        return DisableDecision::Allowed { allowed: true };
    }
    let chain = disable_order(id, facts, &mut HashSet::new());
    let commands: Vec<String> = chain
        .iter()
        .map(|one| format!("claude plugin disable {one}"))
        .collect();
    let quoted: Vec<String> = dependents.iter().map(|one| format!("\"{one}\"")).collect();
    let one = dependents.len() == 1;
    let message = format!(
        "Cannot disable \"{id}\": {} depend{} on it; disable {} first",
        quoted.join(", "),
        if one { "s" } else { "" },
        if one { "it" } else { "them" },
    );
    DisableDecision::Refused {
        allowed: false,
        dependents,
        chain,
        command: commands.join(" && "),
        message,
    }
}

crate::registry::exports! {
    "marketplace/dependency-graph" {
        "recordAt" => ok |id: String, scope: InstallScope, facts: GraphFacts| record_at(&id, scope, &facts).cloned();
        "winningScope" => ok |id: String, facts: GraphFacts| winning_scope(&id, &facts);
        "effectiveEnabled" => ok |id: String, facts: GraphFacts| effective_enabled(&id, &facts);
        "isEnabled" => ok |id: String, facts: GraphFacts| is_enabled(&id, &facts);
        "dependenciesOf" => ok |id: String, facts: GraphFacts| dependencies_of(&id, &facts).to_vec();
        "constraintsOn" => ok |dependency: String, facts: GraphFacts| constraints_on(&dependency, &facts);
        "localCopyOf" => ok |dep: NormalizedDependency, facts: GraphFacts, lookup: Option<CopyLookup>| local_copy_of(&dep, &facts, &lookup.unwrap_or_default());
        "findDependencyCopy" => ok |dep: NormalizedDependency, facts: GraphFacts, lookup: Option<CopyLookup>| find_dependency_copy(&dep, &facts, &lookup.unwrap_or_default());
        "checkDeclaredDependency" => ok |dep: NormalizedDependency, facts: GraphFacts, lookup: Option<CopyLookup>| check_declared_dependency(&dep, &facts, &lookup.unwrap_or_default());
        "dependentsOf" => ok |id: String, facts: GraphFacts| dependents_of(&id, &facts);
        "disableDecision" => ok |id: String, facts: GraphFacts| disable_decision(&id, &facts);
    }
}
