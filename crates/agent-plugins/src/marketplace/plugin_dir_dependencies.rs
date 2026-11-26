//! Port of `src/marketplace/plugin-dir-dependencies.ts`, switched.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::dependencies::{DependencyIssue, Normalized, names_marketplace, normalize_dependency};
use super::dependency_graph::{CopyLookup, GraphFacts, LocalCopy, check_declared_dependency};
use crate::js::JsValue;

/// A plugin a session loads with `--plugin-dir`: its name and its manifest's `dependencies`.
#[derive(Clone, Debug, Deserialize)]
pub struct PluginDirPlugin {
    /// Its name.
    pub name: String,
    /// Its manifest's `dependencies`.
    #[serde(default)]
    pub dependencies: Option<Vec<JsValue>>,
}

/// A `--plugin-dir` plugin that does not load, and the dependency issue that stops it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnloadedPlugin {
    /// Its name.
    pub name: String,
    /// What stops it.
    pub issue: DependencyIssue,
}

fn no_facts() -> GraphFacts {
    GraphFacts {
        installed: Vec::new(),
        enabled: super::dependency_graph::EnabledPlugins::default(),
        catalog: IndexMap::new(),
        allowlists: IndexMap::new(),
        blocked: None,
        local_copies: None,
    }
}

/// A `--plugin-dir` plugin is `<name>@inline`: its bare dependency names resolve there.
const INLINE: &str = "inline";

fn first_issue(
    plugin: &PluginDirPlugin,
    facts: &GraphFacts,
    lookup: &CopyLookup,
) -> Option<DependencyIssue> {
    for item in plugin.dependencies.as_deref().unwrap_or_default() {
        let Normalized::Dependency(dep) = normalize_dependency(item, Some(INLINE), None) else {
            continue;
        };
        let lookup = CopyLookup {
            names_marketplace: Some(names_marketplace(item)),
            ..lookup.clone()
        };
        if let Some(issue) = check_declared_dependency(&dep, facts, &lookup) {
            return Some(issue);
        }
    }
    None
}

/// One pass: the plugins still loading that a dependency now stops.
fn pass(
    plugins: &[PluginDirPlugin],
    unloaded: &IndexMap<String, UnloadedPlugin>,
    facts: &GraphFacts,
    lookup: &CopyLookup,
) -> Vec<UnloadedPlugin> {
    let loading: Vec<&PluginDirPlugin> = plugins
        .iter()
        .filter(|plugin| !unloaded.contains_key(&plugin.name))
        .collect();
    let mut local_copies = facts.local_copies.clone().unwrap_or_default();
    local_copies.extend(loading.iter().map(|plugin| LocalCopy {
        name: plugin.name.clone(),
        enabled: true,
        version: None,
    }));
    let with_copies = GraphFacts {
        local_copies: Some(local_copies),
        ..facts.clone()
    };
    loading
        .into_iter()
        .filter_map(|plugin| {
            first_issue(plugin, &with_copies, lookup).map(|issue| UnloadedPlugin {
                name: plugin.name.clone(),
                issue,
            })
        })
        .collect()
}

/// The `--plugin-dir` plugins of a session that do not load for a dependency (plugins/dependencies.md,
/// "Test a plugin and its dependency locally"): the session's other plugins are the local copies that
/// meet an entry, and a plugin "stops loading whenever the local copy is disabled or absent".
/// `facts` adds what is installed (an installed copy wins, `findDependencyCopy`). Judgments
/// (spec/judgments.md): oracle 2.1.319, a plugin that does not load is no copy, so a plugin that needs
/// it stops loading too; an entry `normalizeDependency` refuses is left to the manifest's validation.
#[must_use]
pub fn plugin_dir_dependency_issues(
    plugins: &[PluginDirPlugin],
    facts: Option<&GraphFacts>,
    lookup: Option<&CopyLookup>,
) -> Vec<UnloadedPlugin> {
    let none = no_facts();
    let facts = facts.unwrap_or(&none);
    let default_lookup = CopyLookup::default();
    let lookup = lookup.unwrap_or(&default_lookup);
    let mut unloaded: IndexMap<String, UnloadedPlugin> = IndexMap::new();
    let mut found = pass(plugins, &unloaded, facts, lookup);
    while !found.is_empty() {
        for one in found {
            unloaded.insert(one.name.clone(), one);
        }
        found = pass(plugins, &unloaded, facts, lookup);
    }
    unloaded.into_values().collect()
}

crate::registry::exports! {
    "marketplace/plugin-dir-dependencies" {
        "pluginDirDependencyIssues" => ok |plugins: Vec<PluginDirPlugin>, facts: Option<GraphFacts>, lookup: Option<CopyLookup>| plugin_dir_dependency_issues(&plugins, facts.as_ref(), lookup.as_ref());
    }
}
