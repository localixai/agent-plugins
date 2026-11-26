//! Port of `src/marketplace/dependency-catalog.ts`, switched.
//!
//! The `dependencies` of a `CatalogPlugin`: its `plugin.json`'s, then its marketplace entry's, in the
//! order `mergeEntryWithManifest` lists them, each with where it was declared, since a refused
//! cross-marketplace dependency refuses the install when the entry declares it and is left out when
//! `plugin.json` does (plugins/dependencies.md, "Depend on a plugin from another marketplace").
//! Bare names resolve against `marketplace`, the declaring plugin's own (manifest-reference.md,
//! `dependencies`). `manifest` is `undefined` for a plugin with no `plugin.json`, whose entry is the
//! manifest, and for one not fetched yet (then the host also sets `manifestUnread`). An item that does
//! not read is an issue at its place in its list and is left out.

use serde::{Deserialize, Serialize};

use super::cross_marketplace::DeclaredIn;
use super::dependencies::{Normalized, names_marketplace, normalize_dependency};
use super::dependency_graph::DeclaredDependency;
use crate::format::issues::PluginIssue;
use crate::js::JsValue;

/// The `dependencies` of a marketplace entry or of a manifest, the only member read of either (a
/// `PluginManifest` is a `JsValue` until its module is in the core).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct WithDependencies {
    /// The declared list.
    #[serde(default)]
    pub dependencies: Option<Vec<JsValue>>,
}

/// What `catalogDependencies` answers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CatalogDependencies {
    /// The dependencies that read.
    pub dependencies: Vec<DeclaredDependency>,
    /// The items that did not.
    pub issues: Vec<PluginIssue>,
}

/// The dependencies of a catalog plugin, each with where it was declared.
#[must_use]
pub fn catalog_dependencies(
    entry: &WithDependencies,
    marketplace: &str,
    manifest: Option<&WithDependencies>,
) -> CatalogDependencies {
    let lists = [
        (
            DeclaredIn::PluginJson,
            manifest.and_then(|manifest| manifest.dependencies.as_deref()),
        ),
        (DeclaredIn::Entry, entry.dependencies.as_deref()),
    ];
    let mut dependencies: Vec<DeclaredDependency> = Vec::new();
    let mut issues: Vec<PluginIssue> = Vec::new();
    for (declared_in, items) in lists {
        for (index, item) in items.unwrap_or_default().iter().enumerate() {
            let path = format!("dependencies[{index}]");
            match normalize_dependency(item, Some(marketplace), Some(&path)) {
                Normalized::Issue(issue) => issues.push(issue),
                Normalized::Dependency(dep) => dependencies.push(DeclaredDependency {
                    dep,
                    declared_in,
                    names_marketplace: names_marketplace(item).then_some(true),
                }),
            }
        }
    }
    CatalogDependencies {
        dependencies,
        issues,
    }
}

crate::registry::exports! {
    "marketplace/dependency-catalog" {
        "catalogDependencies" => ok |entry: WithDependencies, marketplace: String, manifest: Option<WithDependencies>| catalog_dependencies(&entry, &marketplace, manifest.as_ref());
    }
}
