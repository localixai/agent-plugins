//! Port of `src/marketplace/dependency-install-plan.ts`, switched.
//!
//! The transitive install plan of a plugin's dependencies (plugins/install.md, "Plugins with
//! dependencies": "Claude Code also installs and enables the plugin's declared dependencies at the
//! same scope"; plugins/dependencies.md). Pure over `GraphFacts`: the plan says what to install, in
//! which order and at which version, or why nothing is installed; the host does it.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::dependencies::{DependencyIssue, InstalledDependency, check_installed_dependency};
use super::dependency_graph::{CatalogPlugin, GraphFacts, constraints_on, is_enabled, record_at};
use super::dependency_install_walk::{
    LeftOut, Node, Parent, Walk, declared_of, not_listed, walk_from,
};
use super::dependency_tags::{
    Combined, DependencyResolution, Resolution, ResolvedVersion, combine_ranges,
    resolve_dependency_version, resolved_version_label,
};
use super::install_scopes::InstallScope;
use super::semver::satisfies_range;
use super::types::NormalizedDependency;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::types::PluginSource;

/// The operations that install any missing declared dependency (plugins/dependencies.md, "Constrain
/// a dependency that has a non-git source"): `claude plugin install`, `/reload-plugins`, auto-update
/// of the dependent plugin's marketplace, re-running `claude plugin install` on the dependent, and
/// `claude plugin marketplace add`. The docs give the five one behaviour, so the plan does not depend
/// on which runs (judgment, spec/judgments.md); the plan only names it, for the host's report.
pub const DEPENDENCY_INSTALL_OPERATIONS: [&str; 5] = [
    "install",
    "reload-plugins",
    "auto-update",
    "reinstall",
    "marketplace-add",
];

/// What is asked.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    /// The plugin whose dependencies are installed, `name@marketplace`: the one being installed, or an installed one whose dependencies are missing.
    pub root: String,
    /// The scope.
    pub scope: InstallScope,
    /// One of `DEPENDENCY_INSTALL_OPERATIONS`.
    pub operation: String,
    /// The Claude Code version being emulated, for what only newer versions do (a local copy meeting an entry that names a marketplace, from 2.1.242); absent means a current one.
    #[serde(default)]
    pub host_version: Option<String>,
}

/// `install`: not installed; `enable`: installed and disabled; `update`: installed at a version the combined ranges rule out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepAction {
    /// Not installed.
    Install,
    /// Installed and disabled.
    Enable,
    /// Installed outside the ranges.
    Update,
}

/// One step of the plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStep {
    /// `name@marketplace`.
    pub id: String,
    /// What is done.
    pub action: StepAction,
    /// The plugins that declare it, the ones in this plan and the root.
    pub required_by: Vec<String>,
    /// Every range that constrains it: the ones of this plan's plugins and of the installed plugins.
    pub ranges: Vec<String>,
    /// What the ranges resolved against; absent for `enable`, which fetches nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ResolvedVersion>,
    /// The version `claude plugin list` shows (a tag's, with its 12-character commit suffix).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Range problems the fetched copy will have when the plugin loads (a current copy outside a range): the dependent stays disabled.
    pub load_issues: Vec<DependencyIssue>,
}

/// The plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPlan {
    /// The operation.
    pub operation: String,
    /// Dependencies before the plugins that need them. Empty when `issues` is not.
    pub steps: Vec<InstallStep>,
    /// What is left out.
    pub left_out: Vec<LeftOut>,
    /// Why the install is refused whole; nothing is installed or changed then.
    pub issues: Vec<PluginIssue>,
    /// The catalog plugins (`name@marketplace`) the walk reached with `manifestUnread`, in walk order:
    /// their `plugin.json` may declare more. The host fetches them, fills their facts and plans again;
    /// the plan is final, and equal to the plan of a catalog read up front, once `needs` is empty.
    pub needs: Vec<String>,
    /// The plan may be carried out: no issue and nothing needed.
    pub ok: bool,
}

fn distinct(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

/// Every range on a dependency: this plan's plugins' and the installed ones'. A plugin the walk read
/// from the catalog (the root above all) speaks with its new declaration only: its installed range
/// is replaced, not combined ("publish a new version of your plugin with a wider constraint").
fn ranges_for(id: &str, node: &Node, walk: &Walk<'_>) -> Vec<String> {
    let installed = constraints_on(id, walk.facts)
        .into_iter()
        .filter(|one| !walk.walked.contains(&one.by))
        .map(|one| one.range);
    distinct(node.ranges.iter().cloned().chain(installed))
}

fn action_for(id: &str, ranges: &[String], walk: &Walk<'_>) -> Option<StepAction> {
    let Some(record) = record_at(id, walk.scope, walk.facts) else {
        return Some(StepAction::Install);
    };
    let fits = ranges.iter().all(|range| {
        record
            .version
            .as_deref()
            .is_some_and(|version| satisfies_range(version, range))
    });
    if !fits {
        return Some(StepAction::Update);
    }
    (!is_enabled(id, walk.facts)).then_some(StepAction::Enable)
}

fn load_issues(
    dep: &NormalizedDependency,
    resolution: &ResolvedVersion,
    ranges: &[String],
) -> Vec<DependencyIssue> {
    let installed = InstalledDependency {
        enabled: true,
        local: None,
        version: resolution.version.clone(),
    };
    ranges
        .iter()
        .filter_map(|range| {
            let ranged = NormalizedDependency {
                range: Some(range.clone()),
                ..dep.clone()
            };
            check_installed_dependency(&ranged, Some(&installed))
        })
        .collect()
}

/// Whether a step changes the plugin's files: only `enable` does not, so it has nothing to resolve.
const fn fetches(action: StepAction) -> bool {
    match action {
        StepAction::Install | StepAction::Update => true,
        StepAction::Enable => false,
    }
}

/// A step, or the issue that refuses the install.
enum Planned {
    Step(InstallStep),
    Issue(PluginIssue),
}

struct Asked<'a> {
    id: &'a str,
    action: StepAction,
    node: &'a Node,
    ranges: Vec<String>,
    entry: &'a CatalogPlugin,
}

fn requested_by(node: &Node) -> Vec<String> {
    distinct(node.required_by.iter().cloned())
}

/// The fetched step: what the ranges resolve against, the version it shows, and what will fail at load.
fn fetch_step(asked: Asked<'_>) -> Result<Planned, String> {
    let Asked {
        id,
        action,
        node,
        ranges,
        entry,
    } = asked;
    let source =
        PluginSource::deserialize(entry.source.clone()).map_err(|cause| cause.to_string())?;
    let resolved = resolve_dependency_version(&DependencyResolution {
        dep: node.dep.clone(),
        ranges: Some(ranges.clone()),
        source,
        tags: entry.tags.clone().unwrap_or_default(),
        current_copy_version: entry.current_copy_version.clone(),
    })?;
    let resolution = match resolved {
        Resolution::Issue(issue) => return Ok(Planned::Issue(issue)),
        Resolution::Version(resolution) => resolution,
    };
    let commits: Option<&IndexMap<String, String>> = entry.tag_commits.as_ref();
    let commit = resolution
        .tag
        .as_ref()
        .and_then(|tag| commits?.get(tag))
        .map(String::as_str);
    let version = resolved_version_label(&resolution, commit);
    let issues = load_issues(&node.dep, &resolution, &ranges);
    Ok(Planned::Step(InstallStep {
        id: id.to_owned(),
        action,
        required_by: requested_by(node),
        ranges,
        resolution: Some(resolution),
        version,
        load_issues: issues,
    }))
}

/// One step of the plan, or the issue that refuses the install, or nothing for a dependency already met.
fn step_for(id: &str, walk: &Walk<'_>) -> Result<Option<Planned>, String> {
    let Some(node) = walk.nodes.get(id) else {
        return Ok(None);
    };
    let ranges = ranges_for(id, node, walk);
    let entry = walk.facts.catalog.get(id);
    if ranges.len() > 1 {
        let tags = entry
            .and_then(|entry| entry.tags.clone())
            .unwrap_or_default();
        if let Some(Combined::Issue(conflict)) = combine_ranges(&ranges, &tags, &node.dep)? {
            return Ok(Some(Planned::Issue(conflict)));
        }
    }
    let Some(action) = action_for(id, &ranges, walk) else {
        return Ok(None);
    };
    if !fetches(action) {
        return Ok(Some(Planned::Step(InstallStep {
            id: id.to_owned(),
            action,
            required_by: requested_by(node),
            ranges,
            resolution: None,
            version: None,
            load_issues: vec![],
        })));
    }
    match entry {
        None => Ok(Some(Planned::Issue(not_listed(id)))),
        Some(entry) => fetch_step(Asked {
            id,
            action,
            node,
            ranges,
            entry,
        })
        .map(Some),
    }
}

fn id_parts(id: &str) -> (String, String) {
    match id.split_once('@') {
        None => (id.to_owned(), String::new()),
        Some((name, marketplace)) => (name.to_owned(), marketplace.to_owned()),
    }
}

fn refused(
    request: &InstallRequest,
    issues: Vec<PluginIssue>,
    walk: Option<&Walk<'_>>,
) -> InstallPlan {
    InstallPlan {
        operation: request.operation.clone(),
        steps: vec![],
        left_out: walk.map(|walk| walk.left_out.clone()).unwrap_or_default(),
        issues,
        needs: walk.map(|walk| walk.needs.clone()).unwrap_or_default(),
        ok: false,
    }
}

/// The plan that installs, enables or updates the dependencies of `request.root`, transitively and
/// dependencies first. Per dependency: one already installed, enabled and inside every range is met
/// and needs no step, though its own declared dependencies are still planned; installed and disabled is enabled; installed outside the combined ranges is
/// updated; otherwise installed from the catalog. Each step carries its reason (`requiredBy`), the
/// ranges and what they resolve against (plugins/dependencies.md, "How a constraint resolves
/// against tags", "Combine constraints from several plugins"). A dependency in another marketplace
/// follows `crossMarketplaceDecision` with the root plugin's marketplace as the root: one declared in
/// a marketplace entry refuses the install, one in `plugin.json` is left out; a `command` source and
/// a `headersHelper` entry are left to the user and refuse the install until the user installs them
/// by itself (§16.24). A conflict between ranges, a range no tag in an own
/// repository meets, or a dependency its marketplace does not list refuses the whole plan:
/// `steps` is empty and nothing changes ("Plugin A and the dependency stay as they were").
/// Judgments (spec/judgments.md): the docs are silent on cycles and on depth, so each plugin is
/// planned once and the walk has no limit; a cross-marketplace dependency left out does not fail the install; a plugin
/// read from the catalog (the root above all) replaces its installed ranges with its new ones.
/// A catalog plugin with `manifestUnread` is planned from its entry's dependencies and listed in
/// `needs` (judgment `installNeeds`): the host fetches what the plan needs and plans again until
/// nothing is needed. Each round reads more facts and no fewer, and a plugin is needed once, so the
/// rounds end; a plan with `needs` is not final either way (a refusal may yet change).
///
/// # Errors
///
/// What node-semver throws for a range it cannot read, as the TypeScript lets it throw.
pub fn install_closure(
    request: &InstallRequest,
    facts: &GraphFacts,
) -> Result<InstallPlan, String> {
    let Some(declared) = declared_of(&request.root, facts) else {
        let message = format!(
            "Plugin \"{}\" is not in the marketplace catalog",
            request.root
        );
        let issue = error(&format!("dependencies.{}", request.root), &message);
        return Ok(refused(request, vec![issue], None));
    };
    let (name, marketplace) = id_parts(&request.root);
    let mut walk = Walk {
        facts,
        scope: request.scope,
        root: request.root.clone(),
        root_marketplace: facts
            .catalog
            .get(&request.root)
            .map_or_else(|| marketplace.clone(), |entry| entry.marketplace.clone()),
        host_version: request.host_version.clone(),
        walked: vec![],
        nodes: IndexMap::new(),
        order: vec![],
        left_out: vec![],
        issues: vec![],
        needs: vec![],
    };
    let parent = Parent {
        id: request.root.clone(),
        name,
        marketplace,
    };
    walk_from(&parent, &declared, &mut walk);
    let mut failures = walk.issues.clone();
    let mut steps = Vec::new();
    for id in &walk.order {
        match step_for(id, &walk)? {
            Some(Planned::Issue(issue)) => failures.push(issue),
            Some(Planned::Step(step)) => steps.push(step),
            None => {}
        }
    }
    if !failures.is_empty() {
        return Ok(refused(request, failures, Some(&walk)));
    }
    Ok(InstallPlan {
        operation: request.operation.clone(),
        steps,
        left_out: walk.left_out,
        issues: vec![],
        ok: walk.needs.is_empty(),
        needs: walk.needs,
    })
}

crate::registry::exports! {
    "marketplace/dependency-install-plan" {
        "DEPENDENCY_INSTALL_OPERATIONS" => ok || DEPENDENCY_INSTALL_OPERATIONS;
        "installClosure" => fallible |request: InstallRequest, facts: GraphFacts| install_closure(&request, &facts);
    }
}
