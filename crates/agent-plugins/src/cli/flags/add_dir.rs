//! Port of `src/cli/flags/add-dir.ts`, switched.
//!
//! `--add-dir` (cli-reference.md, "CLI flags"; permissions.md, "Additional directories grant file
//! access, not configuration"): extra working directories for reading and editing files. Each path
//! "exists as a directory" or startup fails; "most network paths" cannot be added
//! ("... is a network path, which cannot be added as a working directory", errors.md). The
//! configuration a flag-added directory also loads is a fixed list, and the modes that remove a kind
//! remove it here too: skills, commands and subagents "load through the `project` setting source, so
//! they don't load when you exclude" it, `--bare` "skips the commands and subagents among them" but
//! keeps the skills, and `CLAUDE.local.md` "additionally requires the `local` setting source". What
//! loads is derived from the plan once every flag is combined (`session-flags.ts`).

use serde::{Deserialize, Serialize};

use super::types::{Contribution, Customization, FlagContext, FlagPlatform, PathKind, refuse};
use crate::format::issues::{PluginIssue, warning};
use crate::rules::path::network::is_network_path;
use crate::rules::types::Platform;
use crate::settings::layers::PolicyScope;

/// The configuration types loaded from an `--add-dir` directory.
pub const ADD_DIR_CONFIGURATION: [&str; 7] = [
    "skills",
    "commands",
    "agents",
    "settings:enabledPlugins",
    "settings:extraKnownMarketplaces",
    "claude-md",
    "claude-local-md",
];

/// What decides the load: the combined plan of the flags and the one variable the docs name.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadFacts {
    /// `--bare`.
    pub bare: bool,
    /// `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1`.
    pub claude_md_enabled: bool,
    /// The customizations the modes skip (`SessionPlan.skipped`).
    pub skipped: Vec<Customization>,
    /// The settings scopes that load; absent means every one (`SessionPlan.settingScopes`).
    #[serde(default)]
    pub setting_scopes: Option<Vec<PolicyScope>>,
}

fn skips(facts: &LoadFacts, kind: Customization) -> bool {
    facts.skipped.contains(&kind)
}

fn has_scope(facts: &LoadFacts, scope: PolicyScope) -> bool {
    facts
        .setting_scopes
        .as_ref()
        .is_none_or(|scopes| scopes.contains(&scope))
}

/// CLAUDE.md, `.claude/rules/` and `CLAUDE.local.md`: "Only when `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1` is set".
fn claude_md(facts: &LoadFacts) -> bool {
    facts.claude_md_enabled && !skips(facts, Customization::ClaudeMd)
}

/// One row per kind: whether a flag-added directory loads it. Judgment (spec/judgments.md): the two
/// settings keys load through the `addDir` scope and follow the installed plugins a mode skips.
fn loads(kind: &str, facts: &LoadFacts) -> bool {
    match kind {
        // `--bare` is the one exception: it skips skills as a kind and still loads these.
        "skills" => {
            has_scope(facts, PolicyScope::Project)
                && (facts.bare || !skips(facts, Customization::Skills))
        }
        "commands" => {
            has_scope(facts, PolicyScope::Project) && !skips(facts, Customization::Commands)
        }
        "agents" => has_scope(facts, PolicyScope::Project) && !skips(facts, Customization::Agents),
        "settings:enabledPlugins" | "settings:extraKnownMarketplaces" => {
            has_scope(facts, PolicyScope::AddDir) && !skips(facts, Customization::InstalledPlugins)
        }
        "claude-md" => claude_md(facts),
        "claude-local-md" => claude_md(facts) && has_scope(facts, PolicyScope::Local),
        _ => false,
    }
}

/// The configuration a flag-added directory loads under this plan.
#[must_use]
pub fn add_dir_loads(facts: &LoadFacts) -> Vec<&'static str> {
    ADD_DIR_CONFIGURATION
        .into_iter()
        .filter(|kind| loads(kind, facts))
        .collect()
}

fn not_directory(path: &str) -> String {
    format!("--add-dir: {path} is not a directory")
}

fn network_path(path: &str) -> String {
    format!("{path} is a network path, which cannot be added as a working directory")
}

const fn platform_of(platform: FlagPlatform) -> Platform {
    match platform {
        FlagPlatform::Posix => Platform::Posix,
        FlagPlatform::Windows => Platform::Windows,
    }
}

/// What `--add-dir` contributes, its members in the order the package writes them.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum AddDirContribution {
    /// A path that is not a directory: startup fails.
    Refused(Box<Contribution>),
    /// The directories added, and a warning for each network path left out.
    #[serde(rename_all = "camelCase")]
    Added {
        /// The `additionalDirectories` member of the plan.
        additional_directories: Vec<String>,
        /// The `warnings` member of the plan.
        warnings: Vec<PluginIssue>,
    },
}

/// `--add-dir`'s paths: the directories added, and a warning for each network path left out.
#[must_use]
pub fn add_dir(paths: &[String], context: &FlagContext) -> AddDirContribution {
    let facts = &context.facts;
    let platform = platform_of(facts.platform);
    let network: Vec<&String> = paths
        .iter()
        .filter(|path| is_network_path(path, platform))
        .collect();
    let local: Vec<String> = paths
        .iter()
        .filter(|path| !network.contains(path))
        .cloned()
        .collect();
    let bad: Vec<String> = local
        .iter()
        .filter(|path| facts.directories.get(path.as_str()) != Some(&PathKind::Directory))
        .map(|path| not_directory(path))
        .collect();
    if !bad.is_empty() {
        return AddDirContribution::Refused(Box::new(refuse(bad)));
    }
    AddDirContribution::Added {
        additional_directories: local,
        warnings: network
            .into_iter()
            .map(|path| warning("--add-dir", &network_path(path)))
            .collect(),
    }
}

crate::registry::exports! {
    "cli/flags/add-dir" {
        "ADD_DIR_CONFIGURATION" => ok || ADD_DIR_CONFIGURATION;
        "addDirLoads" => ok |facts: LoadFacts| add_dir_loads(&facts);
    }
}
