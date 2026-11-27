//! Ported from `src/marketplace/dependency-install-walk.ts`; only the core uses it now, so it has no wrapper.
//!
//! The walk over a plugin's declared dependencies that `installClosure` plans from
//! (plugins/dependencies.md, "Depend on a plugin from another marketplace", "Constrain a dependency
//! that has a non-git source"): it collects every dependency with everyone who asks for it and every
//! range on it, depth first, and leaves out what Claude Code does not install.
//!
//! The TypeScript's `walkFrom` fills the walk it is given in place, so it never crossed; the walk is
//! the plan's own value here.

use indexmap::IndexMap;
use serde::Serialize;

use super::cross_marketplace::{
    CrossMarketplaceDecision, CrossMarketplaceQuestion, DeclaredIn, RefusalEffect, RequiredBy,
    RootMarketplace, cross_marketplace_decision,
};
use super::dependencies::dependency_id;
use super::dependency_graph::{
    CatalogPlugin, CopyLookup, DeclaredDependency, GraphFacts, dependencies_of, local_copy_of,
    record_at,
};
use super::install_scopes::{InstallScope, install_command};
use super::types::NormalizedDependency;
use crate::format::issues::{PluginIssue, error};
use crate::js::JsValue;
use crate::registry::Set;

/// Why Claude Code does not install a dependency itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LeftOutReason {
    /// A `command` source.
    CommandSource,
    /// A `headersHelper`.
    HeadersHelper,
    /// Another marketplace that is not allowed.
    CrossMarketplace,
}

/// A dependency the install leaves out. A `command` or `headersHelper` one also refuses the install
/// until the user installs it by itself (§16.24); a cross-marketplace `plugin.json` one does not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LeftOut {
    /// The dependency.
    pub id: String,
    /// Why.
    pub reason: LeftOutReason,
    /// What the user is told.
    pub message: String,
    /// What the user runs first, when there is something.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// Node of the walk: one dependency id with everything that asks for it.
#[derive(Clone, Debug)]
pub struct Node {
    /// The dependency.
    pub dep: NormalizedDependency,
    /// Who asks for it.
    pub required_by: Vec<String>,
    /// The ranges on it.
    pub ranges: Vec<String>,
}

/// The plugin whose declarations are walked.
#[derive(Clone, Debug)]
pub struct Parent {
    /// `name@marketplace`.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its marketplace.
    pub marketplace: String,
}

/// The walk's state.
pub struct Walk<'f> {
    /// What is known.
    pub facts: &'f GraphFacts,
    /// The scope of the install.
    pub scope: InstallScope,
    /// The plugin whose dependencies are walked: a cycle back to it ends there.
    pub root: String,
    /// Its marketplace.
    pub root_marketplace: String,
    /// The Claude Code version being emulated; absent means a current one (see `CopyLookup`).
    pub host_version: Option<String>,
    /// The plugins whose declarations were read: their installed constraints are replaced by what was read.
    pub walked: Vec<String>,
    /// Each dependency id with who asks for it.
    pub nodes: IndexMap<String, Node>,
    /// Ids to act on or check, dependencies first.
    pub order: Vec<String>,
    /// What is left out.
    pub left_out: Vec<LeftOut>,
    /// What refuses the install.
    pub issues: Vec<PluginIssue>,
    /// Catalog plugins whose declarations the walk read while their `plugin.json` was unread, in walk order.
    pub needs: Vec<String>,
}

/// Which source kinds Claude Code never installs as a dependency (a registry typed over the kinds).
fn never_installed(kind: Option<&str>) -> Option<LeftOutReason> {
    (kind == Some("command")).then_some(LeftOutReason::CommandSource)
}

const fn left_text(reason: LeftOutReason) -> &'static str {
    match reason {
        LeftOutReason::CommandSource => {
            "has a command source, which Claude Code never installs as a dependency"
        }
        LeftOutReason::HeadersHelper => {
            "has a headersHelper, which Claude Code never runs for a dependency"
        }
        LeftOutReason::CrossMarketplace => "is in another marketplace that is not allowed",
    }
}

/// A `headersHelper` wins over the source, as in the updates module.
fn left_reason(entry: &CatalogPlugin) -> Option<LeftOutReason> {
    if entry.headers_helper.is_some() {
        return Some(LeftOutReason::HeadersHelper);
    }
    never_installed(entry.source.get("kind").and_then(JsValue::as_str))
}

fn leave(id: &str, reason: LeftOutReason, message: Option<String>) -> LeftOut {
    let command = install_command(id);
    let user = reason != LeftOutReason::CrossMarketplace;
    LeftOut {
        id: id.to_owned(),
        reason,
        message: message.unwrap_or_else(|| {
            format!(
                "Dependency \"{id}\" {}; install it first: {command}",
                left_text(reason)
            )
        }),
        command: user.then_some(command),
    }
}

/// The refusal for a dependency its marketplace does not list.
#[must_use]
pub fn not_listed(id: &str) -> PluginIssue {
    error(
        &format!("dependencies.{id}"),
        &format!("Dependency \"{id}\" is not installed and its marketplace does not list it"),
    )
}

fn node_for(walk: &mut Walk<'_>, dep: &NormalizedDependency, by: &str) {
    let id = dependency_id(dep);
    let node = walk.nodes.entry(id).or_insert_with(|| Node {
        dep: dep.clone(),
        required_by: vec![],
        ranges: vec![],
    });
    node.required_by.push(by.to_owned());
    if let Some(range) = &dep.range {
        node.ranges.push(range.clone());
    }
}

/// The declarations of `id`: the catalog's, else its install record's.
#[must_use]
pub fn declared_of(id: &str, facts: &GraphFacts) -> Option<Vec<DeclaredDependency>> {
    if let Some(entry) = facts.catalog.get(id) {
        return Some(entry.dependencies.clone());
    }
    let installed = facts.installed.iter().any(|record| record.id == id);
    installed.then(|| {
        dependencies_of(id, facts)
            .iter()
            .map(|dep| DeclaredDependency {
                dep: dep.clone(),
                declared_in: DeclaredIn::PluginJson,
                names_marketplace: None,
            })
            .collect()
    })
}

/// What the walk does with a dependency: walk into its declared dependencies, or leave it out (`None`).
type Children = Option<Vec<DeclaredDependency>>;

/// The allowlist (plugins/dependencies.md, "Depend on a plugin from another marketplace"); a refusal is recorded as the declaration decides.
fn cross_allowed(declared: &DeclaredDependency, parent: &Parent, walk: &mut Walk<'_>) -> bool {
    let DeclaredDependency {
        dep, declared_in, ..
    } = declared;
    let id = dependency_id(dep);
    let mut enabled_at_scope: Vec<String> = Vec::new();
    for one in &walk.facts.installed {
        let enabled = walk.facts.enabled.value(walk.scope, &one.id) == Some(true);
        if one.scope == walk.scope && enabled && !enabled_at_scope.contains(&one.id) {
            enabled_at_scope.push(one.id.clone());
        }
    }
    let decision = cross_marketplace_decision(&CrossMarketplaceQuestion {
        dep: dep.clone(),
        required_by: RequiredBy {
            name: parent.name.clone(),
            marketplace: Some(parent.marketplace.clone()),
        },
        root: RootMarketplace {
            name: walk.root_marketplace.clone(),
            allow_cross_marketplace_dependencies_on: walk
                .facts
                .allowlists
                .get(&walk.root_marketplace)
                .cloned()
                .unwrap_or_default(),
        },
        declaring_marketplace: None,
        enabled_at_scope: Set {
            members: enabled_at_scope,
        },
        declared_in: *declared_in,
    });
    match decision {
        CrossMarketplaceDecision::Allowed { allowed } => allowed,
        CrossMarketplaceDecision::Refused {
            effect, message, ..
        } => {
            if effect == RefusalEffect::RefuseInstall {
                walk.issues
                    .push(error(&format!("dependencies.{id}"), &message));
            } else {
                walk.left_out
                    .push(leave(&id, LeftOutReason::CrossMarketplace, Some(message)));
            }
            false
        }
    }
}

/// A dependency to install: the catalog must list it, and Claude Code must be willing to install it.
fn from_catalog(id: &str, walk: &mut Walk<'_>) -> Children {
    let Some(entry) = walk.facts.catalog.get(id) else {
        walk.issues.push(not_listed(id));
        return None;
    };
    let Some(reason) = left_reason(entry) else {
        return Some(entry.dependencies.clone());
    };
    // The user installs it by itself first; until then the dependent fails to install (§16.24).
    let left = leave(id, reason, None);
    walk.issues
        .push(error(&format!("dependencies.{id}"), &left.message));
    walk.left_out.push(left);
    None
}

/// Where a `--plugin-dir` copy meets this entry: the emulated host version and whether the entry names a marketplace.
fn copy_lookup(declared: &DeclaredDependency, walk: &Walk<'_>) -> CopyLookup {
    CopyLookup {
        host_version: walk.host_version.clone(),
        names_marketplace: (declared.names_marketplace == Some(true)).then_some(true),
    }
}

fn has_local_copy(declared: &DeclaredDependency, walk: &Walk<'_>) -> bool {
    local_copy_of(&declared.dep, walk.facts, &copy_lookup(declared, walk)).is_some()
}

/// What the walk does with one declared dependency. One installed and enabled at the scope (its own
/// `enabledPlugins` says so: another scope's value does not count) is met: it
/// gets no step, but its own declared dependencies are walked, since a re-run installs "any missing
/// declared dependency" (plugins/dependencies.md, "Constrain a dependency that has a non-git
/// source").
fn children_of(declared: &DeclaredDependency, parent: &Parent, walk: &mut Walk<'_>) -> Children {
    let id = dependency_id(&declared.dep);
    let facts = walk.facts;
    let record = record_at(&id, walk.scope, facts);
    if record.is_some() && facts.enabled.value(walk.scope, &id) == Some(true) {
        return Some(declared_of(&id, facts).unwrap_or_default());
    }
    if record.is_none() && has_local_copy(declared, walk) {
        return None;
    }
    if !cross_allowed(declared, parent, walk) {
        return None;
    }
    if record.is_none() {
        from_catalog(&id, walk)
    } else {
        declared_of(&id, facts)
    }
}

/// Depth-first over declared dependencies, those of a met dependency too; a dependency met again only adds its asker and range, so a cycle ends.
pub fn walk_from(parent: &Parent, declared: &[DeclaredDependency], walk: &mut Walk<'_>) {
    if !walk.walked.contains(&parent.id) {
        walk.walked.push(parent.id.clone());
    }
    // Every declaration list the walk reads is the catalog's when the catalog lists the plugin
    // (`declaredOf`, `fromCatalog`), so an unread catalog plugin here is one whose list may grow.
    if walk
        .facts
        .catalog
        .get(&parent.id)
        .is_some_and(|entry| entry.manifest_unread == Some(true))
    {
        walk.needs.push(parent.id.clone());
    }
    for one in declared {
        let dep = &one.dep;
        let id = dependency_id(dep);
        if id == walk.root || id == parent.id {
            continue;
        }
        let known = walk.nodes.contains_key(&id);
        node_for(walk, dep, &parent.id);
        if known {
            continue;
        }
        let Some(children) = children_of(one, parent, walk) else {
            continue;
        };
        let child = Parent {
            id: id.clone(),
            name: dep.name.clone(),
            marketplace: dep
                .marketplace
                .clone()
                .unwrap_or_else(|| parent.marketplace.clone()),
        };
        walk_from(&child, &children, walk);
        walk.order.push(id);
    }
}
