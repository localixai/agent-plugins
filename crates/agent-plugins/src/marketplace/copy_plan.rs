//! Port of `src/marketplace/copy-plan.ts`, switched.
//!
//! What copying a plugin tree into the cache does with each entry (plugins/host-marketplace.md,
//! "Share files within a marketplace with symlinks"; plugins/marketplace-reference.md, "Output that
//! fails the install or update"): a plan over the entries a host listed without following links,
//! and over the real paths it resolved for each link. Nothing is read or written here.
//!
//! A link is handled "by where the target resolves": within the plugin it is kept as a relative
//! link, elsewhere in the marketplace it is dereferenced (the target's content is copied in its
//! place), outside the marketplace it is skipped. "For plugins installed from a local path, or from
//! a `command` source whose `mode` is the default `copy`", which have no marketplace to
//! dereference into, only a link within the plugin's own directory is kept.
//!
//! A dereferenced directory is the host's to list and plan again, with its children's paths below
//! the link's and its real path added to `TreeRoots.copying`. A link to a directory that is, or
//! contains, one being copied (the marketplace root, the plugin's own parent, a sibling that links
//! back) would copy the same tree again without end, so it is skipped as a `cycle` (spec/
//! judgments.md). A link whose target does not exist is skipped (judgment).

use serde::{Deserialize, Serialize};

use super::command_directory::{CopyLimits, too_large, too_many_entries};
use crate::format::issues::{PluginIssue, error};
use crate::format::link_rule::{LinkReach, LinkReachInput, link_reach, relative_from};
use crate::format::path_text::{is_within, without_trailing_slashes};

/// Where a link finally resolves.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkTarget {
    /// The real path.
    pub path: String,
    /// It is a directory.
    pub is_directory: bool,
}

/// One entry of the tree (`TreeEntry`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    /// Where the entry lands, relative to the plugin root and `/`-separated.
    pub path: String,
    /// It is a directory.
    pub is_directory: bool,
    /// It is a link.
    pub is_symlink: bool,
    /// The size of a file; 0 for a directory or a link.
    pub bytes: f64,
    /// For a link: the real path it finally resolves to; absent when it points at nothing.
    #[serde(default)]
    pub target: Option<LinkTarget>,
}

/// The roots of the copy (`TreeRoots`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeRoots {
    /// The real path of the plugin directory, spelled as the targets are.
    pub plugin_root: String,
    /// The marketplace's real path; absent for a plugin installed from a local path or in copy mode.
    #[serde(default)]
    pub marketplace_root: Option<String>,
    /// The real paths of the directories already being copied above the one planned now: each
    /// dereferenced directory the host went into, in order. The plugin root is always one.
    #[serde(default)]
    pub copying: Option<Vec<String>>,
}

/// A link kept, pointing anew.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Relink {
    /// The link.
    pub path: String,
    /// The link's new target, relative to the folder the link is in and `/`-separated.
    pub target: String,
}

/// A link replaced by what it points at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dereference {
    /// The link.
    pub path: String,
    /// The real path whose content is copied where the link was.
    pub from: String,
    /// It is a directory.
    pub is_directory: bool,
}

/// Why a link is skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkipReason {
    /// It leaves the marketplace.
    OutsideMarketplace,
    /// It points at nothing.
    Dangling,
    /// It would copy a tree being copied again.
    Cycle,
}

/// A link skipped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Skipped {
    /// The link.
    pub path: String,
    /// Why.
    pub reason: SkipReason,
}

/// The plan (`TreePlan`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TreePlan {
    /// Files and directories copied as they are.
    pub copy: Vec<String>,
    /// Links kept.
    pub relinks: Vec<Relink>,
    /// Links replaced by their content.
    pub dereferences: Vec<Dereference>,
    /// Links skipped.
    pub skipped: Vec<Skipped>,
    /// The copy cannot be made: it is too large.
    pub refusals: Vec<PluginIssue>,
}

type Link = TreeEntry;

enum Decision {
    Relink(String),
    Dereference(Dereference),
    Skip(SkipReason),
}

fn directory_of(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    parts
        .get(..parts.len().saturating_sub(1))
        .unwrap_or_default()
        .join("/")
}

fn within_plugin(link: &Link, target: &LinkTarget, roots: &TreeRoots) -> Decision {
    let root = without_trailing_slashes(&roots.plugin_root);
    let below = target.path.get(root.len()..).unwrap_or_default();
    let rel = below.strip_prefix('/').unwrap_or(below);
    Decision::Relink(relative_from(&directory_of(&link.path), rel))
}

/// A directory that is, or contains, one being copied: copying it would copy that one again.
fn closes_cycle(target: &LinkTarget, roots: &TreeRoots) -> bool {
    target.is_directory
        && std::iter::once(&roots.plugin_root)
            .chain(roots.copying.iter().flatten())
            .any(|open| is_within(&target.path, open))
}

/// What each reach does with a link: the rule of the docs' three bullets, one entry per reach.
fn by_reach(reach: LinkReach, link: &Link, target: &LinkTarget, roots: &TreeRoots) -> Decision {
    match reach {
        LinkReach::WithinPlugin => within_plugin(link, target, roots),
        LinkReach::WithinMarketplace => {
            if closes_cycle(target, roots) {
                Decision::Skip(SkipReason::Cycle)
            } else {
                Decision::Dereference(Dereference {
                    path: link.path.clone(),
                    from: target.path.clone(),
                    is_directory: target.is_directory,
                })
            }
        }
        LinkReach::Outside => Decision::Skip(SkipReason::OutsideMarketplace),
    }
}

fn decide_link(link: &Link, roots: &TreeRoots) -> Decision {
    let Some(target) = &link.target else {
        return Decision::Skip(SkipReason::Dangling);
    };
    let reach = link_reach(&LinkReachInput {
        target: target.path.clone(),
        plugin_root: roots.plugin_root.clone(),
        marketplace_root: roots.marketplace_root.clone(),
    });
    by_reach(reach, link, target, roots)
}

/// The refusal when what is copied is "larger than 256 MiB or has more than 20,000 entries".
fn limit_refusals(entries: &[TreeEntry], limits: &CopyLimits) -> Vec<PluginIssue> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a count of entries a host listed"
    )]
    let count = entries.len() as f64;
    if count > limits.max_entries {
        return vec![too_many_entries(limits.max_entries)];
    }
    let bytes: f64 = entries.iter().map(|one| one.bytes).sum();
    if bytes > limits.max_bytes {
        vec![too_large(limits.max_bytes)]
    } else {
        vec![]
    }
}

/// A plan of copying `entries`; `limits` apply to a tree that is copied whole (a `command` source in copy mode).
#[must_use]
pub fn plan_tree_copy(
    entries: &[TreeEntry],
    roots: &TreeRoots,
    limits: Option<&CopyLimits>,
) -> TreePlan {
    let mut plan = TreePlan {
        copy: vec![],
        relinks: vec![],
        dereferences: vec![],
        skipped: vec![],
        refusals: vec![],
    };
    for entry in entries {
        if !entry.is_symlink {
            plan.copy.push(entry.path.clone());
            continue;
        }
        match decide_link(entry, roots) {
            Decision::Relink(target) => plan.relinks.push(Relink {
                path: entry.path.clone(),
                target,
            }),
            Decision::Dereference(dereference) => plan.dereferences.push(dereference),
            Decision::Skip(reason) => plan.skipped.push(Skipped {
                path: entry.path.clone(),
                reason,
            }),
        }
    }
    plan.refusals = limits.map_or_else(Vec::new, |limits| limit_refusals(entries, limits));
    plan
}

/// What `pluginRootIssue` is given.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRootInput {
    /// The entry's path as the marketplace wrote it.
    pub entry_path: String,
    /// The plugin directory's real path.
    pub plugin_root: String,
    /// The marketplace's real path.
    pub marketplace_root: String,
}

/// The refusal for a plugin directory of a fetched marketplace that is, or passes through, a link
/// that resolves outside the marketplace ("an entry of a fetched marketplace that resolves or opens
/// outside its tree", plugins/troubleshooting.md): the listing line `claude plugin list` shows.
#[must_use]
pub fn plugin_root_issue(input: &PluginRootInput) -> Option<PluginIssue> {
    if is_within(&input.marketplace_root, &input.plugin_root) {
        return None;
    }
    Some(error(
        "source",
        &format!(
            "Plugin source path refused: {} does not stay inside its marketplace directory. Check that the marketplace entry has a plain relative path.",
            input.entry_path
        ),
    ))
}

crate::registry::exports! {
    "marketplace/copy-plan" {
        "planTreeCopy" => ok |entries: Vec<TreeEntry>, roots: TreeRoots, limits: Option<CopyLimits>|
            plan_tree_copy(&entries, &roots, limits.as_ref());
        "pluginRootIssue" => ok |input: PluginRootInput| plugin_root_issue(&input);
    }
}
