//! Port of `src/marketplace/submodules.ts`, switched.
//!
//! Which git submodules of the marketplace repository a relative-path plugin needs (plugins/
//! marketplace-reference.md, "Git submodules in the marketplace repository", Claude Code v2.1.295
//! and later): pure, the paths in `.gitmodules` and what the plugin's files say in, the paths to
//! fetch out. The host reads the files and runs the fetch; the kit only decides.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::types::SubmoduleRule;
use crate::format::path_text::{is_within, without_trailing_slashes};
use crate::js::{Pattern, SPACE_CLASS, split_line_breaks};

/// How a marketplace was added (`src/format/marketplace/types.ts`, which is not ported yet and
/// owns this type).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceSourceKind {
    /// `github`.
    #[serde(rename = "github")]
    Github,
    /// `git`.
    #[serde(rename = "git")]
    Git,
    /// `url`.
    #[serde(rename = "url")]
    Url,
    /// `file`.
    #[serde(rename = "file")]
    File,
    /// `directory`.
    #[serde(rename = "directory")]
    Directory,
    /// `npm`.
    #[serde(rename = "npm")]
    Npm,
    /// `skills-dir`.
    #[serde(rename = "skills-dir")]
    SkillsDir,
    /// `hostPattern`.
    #[serde(rename = "hostPattern")]
    HostPattern,
    /// `pathPattern`.
    #[serde(rename = "pathPattern")]
    PathPattern,
    /// `settings`.
    #[serde(rename = "settings")]
    Settings,
}

/// How a marketplace was added: only the members this module reads (`src/format/marketplace/types.ts`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceSource {
    /// How it was added.
    pub kind: MarketplaceSourceKind,
    /// The paths of a sparse checkout.
    #[serde(default)]
    pub sparse_paths: Option<Vec<String>>,
}

/// A marketplace clone has submodules to fetch only when it is added from git.
const fn cloned(kind: MarketplaceSourceKind) -> bool {
    match kind {
        MarketplaceSourceKind::Github | MarketplaceSourceKind::Git => true,
        MarketplaceSourceKind::Url
        | MarketplaceSourceKind::File
        | MarketplaceSourceKind::Directory
        | MarketplaceSourceKind::Npm
        | MarketplaceSourceKind::SkillsDir
        | MarketplaceSourceKind::HostPattern
        | MarketplaceSourceKind::PathPattern
        | MarketplaceSourceKind::Settings => false,
    }
}

fn has_sparse_paths(origin: &MarketplaceSource) -> bool {
    origin
        .sparse_paths
        .as_ref()
        .is_some_and(|paths| !paths.is_empty())
}

/// The rule for a plugin at `path` ("." at the root) of a marketplace added as `origin`: a sparse
/// checkout "fetches no submodule", a marketplace that was not cloned has none to fetch, a plugin
/// at the root gets every submodule or none, any other plugin only the ones it touches.
#[must_use]
pub fn submodule_rule_for(path: &str, origin: Option<&MarketplaceSource>) -> SubmoduleRule {
    if origin.is_some_and(|origin| !cloned(origin.kind) || has_sparse_paths(origin)) {
        return SubmoduleRule::None;
    }
    if path == "." {
        SubmoduleRule::WholeRepository
    } else {
        SubmoduleRule::PluginSubtree
    }
}

/// What the plugin's repository and files say about its submodules; every path is relative to the repository root.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmoduleEvidence {
    /// The `path` of every submodule in `.gitmodules` (`submodulePaths` reads them).
    pub submodules: Vec<String>,
    /// The plugin's directory: `.` at the root.
    pub plugin: String,
    /// Where each symbolic link committed to the repository points.
    pub link_targets: Vec<String>,
    /// Root plugin: the directories at the root that hold plugin components (`skills/`, `hooks/`, `bin/`).
    pub component_directories: Vec<String>,
    /// Root plugin: the paths written in the marketplace entry, `plugin.json` and config files (`.mcp.json`, `hooks/hooks.json`).
    pub config_paths: Vec<String>,
    /// Root plugin: the text of the config files and of the files under the component directories.
    pub texts: Vec<String>,
    /// The kit "can't tell which ones hold plugin files": a `SKILL.md` at the root, or an unreadable marketplace file, config file or `.gitmodules`.
    pub undecidable: bool,
}

/// A path as a plain `a/b`: no leading `./`, no trailing `/`; the root is `.`.
fn tidy(path: &str) -> String {
    let slashed = path.replace('\\', "/");
    let mut rest = slashed.as_str();
    while let Some(after) = rest.strip_prefix("./") {
        rest = after;
    }
    let trimmed = without_trailing_slashes(rest);
    if trimmed.is_empty() {
        ".".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `outer` is `inner` or a directory above it.
fn holds(outer: &str, inner: &str) -> bool {
    let a = tidy(outer);
    let b = tidy(inner);
    a == "." || is_within(&a, &b)
}

fn overlaps(a: &str, b: &str) -> bool {
    holds(a, b) || holds(b, a)
}

/// A plugin in a subdirectory: it is in the submodule, the submodule is in it, or a link points into the submodule.
fn touches_subtree(submodule: &str, evidence: &SubmoduleEvidence) -> bool {
    overlaps(submodule, &evidence.plugin)
        || evidence
            .link_targets
            .iter()
            .any(|target| holds(submodule, target))
}

/// A plugin at the root: a component directory, a configured path or a link points into the submodule, or a text names it.
fn touches_root(submodule: &str, evidence: &SubmoduleEvidence) -> bool {
    let named = tidy(submodule);
    evidence
        .component_directories
        .iter()
        .any(|directory| overlaps(submodule, directory))
        || evidence
            .config_paths
            .iter()
            .any(|path| overlaps(submodule, path))
        || evidence
            .link_targets
            .iter()
            .any(|target| holds(submodule, target))
        || evidence.texts.iter().any(|text| text.contains(&named))
}

/// Every submodule when the kit cannot tell, else the ones `touches` picks (all or none for a root plugin).
fn in_subtree(evidence: &SubmoduleEvidence) -> Vec<String> {
    if evidence.undecidable {
        return evidence.submodules.iter().map(|path| tidy(path)).collect();
    }
    evidence
        .submodules
        .iter()
        .filter(|path| touches_subtree(path, evidence))
        .map(|path| tidy(path))
        .collect()
}

fn in_repository(evidence: &SubmoduleEvidence) -> Vec<String> {
    let all: Vec<String> = evidence.submodules.iter().map(|path| tidy(path)).collect();
    if evidence.undecidable {
        return all;
    }
    if evidence
        .submodules
        .iter()
        .any(|path| touches_root(path, evidence))
    {
        all
    } else {
        Vec::new()
    }
}

/// The submodules to fetch, in `.gitmodules` order. A root plugin gets all of them or none; when the
/// kit cannot tell, every one (unless the rule is `none`). A path counts when it is the submodule or
/// inside it, and also when it holds the submodule (spec/judgments.md): the rule errs toward fetching.
#[must_use]
pub fn select_submodules(rule: SubmoduleRule, evidence: &SubmoduleEvidence) -> Vec<String> {
    match rule {
        SubmoduleRule::None => Vec::new(),
        SubmoduleRule::PluginSubtree => in_subtree(evidence),
        SubmoduleRule::WholeRepository => in_repository(evidence),
    }
}

/// `/^\s*path\s*=\s*(.+?)\s*$/i`: `.` is any character but a line terminator, and the letters fold
/// by ASCII only.
static MODULE_PATH: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"^[{SPACE_CLASS}]*(?i-u:path)[{SPACE_CLASS}]*=[{SPACE_CLASS}]*([^\n\r\u{{2028}}\u{{2029}}]+?)[{SPACE_CLASS}]*$"
    ))
    .ok()
});

/// The `path` of each submodule in the text of a `.gitmodules` file, in file order.
#[must_use]
pub fn submodule_paths(gitmodules: &str) -> Vec<String> {
    let Some(pattern) = MODULE_PATH.as_ref() else {
        return Vec::new();
    };
    split_line_breaks(gitmodules)
        .into_iter()
        .filter_map(|line| pattern.captures(line)?.get(1).map(|found| found.as_str()))
        .map(tidy)
        .collect()
}

crate::registry::exports! {
    "marketplace/submodules" {
        "submoduleRuleFor" => ok |path: String, origin: Option<MarketplaceSource>| submodule_rule_for(&path, origin.as_ref());
        "selectSubmodules" => ok |rule: SubmoduleRule, evidence: SubmoduleEvidence| select_submodules(rule, &evidence);
        "submodulePaths" => ok |gitmodules: String| submodule_paths(&gitmodules);
    }
}
