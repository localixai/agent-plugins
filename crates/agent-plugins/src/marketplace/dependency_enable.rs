//! Port of `src/marketplace/dependency-enable.ts`, switched.
//!
//! Enabling a plugin that declares dependencies (plugins/install.md, "Plugins with dependencies":
//! "Claude Code also enables the plugin's dependencies that are installed but disabled. If a declared
//! dependency isn't installed, the enable fails and the message tells you to install it first";
//! plugins/cli-reference.md, `plugin enable`: the three ways it fails). Pure: the host writes the
//! settings the plan names.

use std::collections::HashSet;

use serde::Serialize;

use super::dependencies::dependency_id;
use super::dependency_graph::{GraphFacts, dependencies_of, is_enabled, records_of};
use super::install_scopes::{InstallScope, SCOPES_BY_PRECEDENCE, install_command};
use crate::format::issues::{PluginIssue, error};

/// Why a dependency stops an enable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum EnableBlock {
    /// `not-installed`.
    #[serde(rename = "not-installed")]
    NotInstalled,
    /// `policy-blocked`.
    #[serde(rename = "policy-blocked")]
    PolicyBlocked,
    /// `disabled-at-higher-scope`.
    #[serde(rename = "disabled-at-higher-scope")]
    DisabledAtHigherScope,
}

/// One dependency that stops the enable, with what the user can do about it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EnableIssue {
    /// The issue itself.
    #[serde(flatten)]
    pub issue: PluginIssue,
    /// Why it stops the enable.
    pub block: EnableBlock,
    /// The dependency's id.
    pub dependency: String,
    /// `not-installed`: the `claude plugin install` command for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// `disabled-at-higher-scope`: the scope that holds the `false`, where to enable it or to pass as `--scope`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<InstallScope>,
}

/// What enabling does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EnablePlan {
    /// Whether nothing stops it.
    pub ok: bool,
    /// The dependencies to enable (installed but disabled), each after its own dependencies.
    pub enable: Vec<String>,
    /// Every dependency that stops the enable; nothing is enabled when there is one.
    pub issues: Vec<EnableIssue>,
}

fn issue_at(
    dependency: &str,
    block: EnableBlock,
    message: &str,
    command: Option<String>,
    scope: Option<InstallScope>,
) -> EnableIssue {
    EnableIssue {
        issue: error(&format!("dependencies.{dependency}"), message),
        block,
        dependency: dependency.to_owned(),
        command,
        scope,
    }
}

/// The scope that holds a `false` for the dependency and wins over `target`, if there is one.
fn overridden_above(
    dependency: &str,
    target: InstallScope,
    facts: &GraphFacts,
) -> Option<InstallScope> {
    let end = SCOPES_BY_PRECEDENCE
        .iter()
        .position(|scope| *scope == target)
        .unwrap_or(0);
    SCOPES_BY_PRECEDENCE
        .iter()
        .take(end)
        .copied()
        .find(|scope| facts.enabled.value(*scope, dependency) == Some(false))
}

/// What stops one dependency from being enabled at `target`, or `None`.
fn block_of(dependency: &str, target: InstallScope, facts: &GraphFacts) -> Option<EnableIssue> {
    if facts
        .blocked
        .as_ref()
        .is_some_and(|blocked| blocked.members.iter().any(|one| one == dependency))
    {
        return Some(issue_at(
            dependency,
            EnableBlock::PolicyBlocked,
            &format!("Dependency \"{dependency}\" is blocked by your organization's plugin policy"),
            None,
            None,
        ));
    }
    if records_of(dependency, facts).is_empty() {
        let command = install_command(dependency);
        return Some(issue_at(
            dependency,
            EnableBlock::NotInstalled,
            &format!("Dependency \"{dependency}\" is not installed; install it first: {command}"),
            Some(command),
            None,
        ));
    }
    let scope = overridden_above(dependency, target, facts)?;
    let (above, below) = (scope.as_str(), target.as_str());
    Some(issue_at(
        dependency,
        EnableBlock::DisabledAtHigherScope,
        &format!(
            "Dependency \"{dependency}\" is set to false at {above} scope, which takes precedence over {below}; enable it at {above} scope or pass --scope {above}"
        ),
        None,
        Some(scope),
    ))
}

struct Walk<'f> {
    target: InstallScope,
    facts: &'f GraphFacts,
    seen: HashSet<String>,
    enable: Vec<String>,
    issues: Vec<EnableIssue>,
}

fn visit(id: &str, walk: &mut Walk<'_>) {
    for dep in dependencies_of(id, walk.facts) {
        let dependency = dependency_id(dep);
        if walk.seen.contains(&dependency) {
            continue;
        }
        walk.seen.insert(dependency.clone());
        if let Some(block) = block_of(&dependency, walk.target, walk.facts) {
            walk.issues.push(block);
            continue;
        }
        visit(&dependency, walk);
        if !is_enabled(&dependency, walk.facts) {
            walk.enable.push(dependency);
        }
    }
}

/// What enabling `id` at `target` does to its dependencies, transitively: the ones installed but
/// disabled are enabled, dependencies first; a dependency that is not installed, is blocked by the
/// plugin policy or is `false` at a scope that wins over `target` fails the enable (and so enables
/// nothing). Judgment: the cascade follows dependencies of dependencies, which the docs imply by
/// "the plugin's dependencies" being enabled so that the plugin loads; a dependency already enabled
/// by a winning scope is left alone, and one installed at any scope counts as installed.
#[must_use]
pub fn enable_plan(id: &str, target: InstallScope, facts: &GraphFacts) -> EnablePlan {
    let mut walk = Walk {
        target,
        facts,
        seen: HashSet::from([id.to_owned()]),
        enable: Vec::new(),
        issues: Vec::new(),
    };
    visit(id, &mut walk);
    let ok = walk.issues.is_empty();
    EnablePlan {
        ok,
        enable: if ok { walk.enable } else { Vec::new() },
        issues: walk.issues,
    }
}

crate::registry::exports! {
    "marketplace/dependency-enable" {
        "enablePlan" => ok |id: String, target: InstallScope, facts: GraphFacts| enable_plan(&id, target, &facts);
    }
}
