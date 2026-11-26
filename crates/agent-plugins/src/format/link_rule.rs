//! Port of `src/format/link-rule.ts`, switched.
//!
//! Where a symbolic link leads, judged against the plugin and its marketplace (plugins/host-
//! marketplace.md, "Share files within a marketplace with symlinks"; plugins/loading.md, "Paths
//! that escape the plugin directory"). Pure over paths the host resolved (real paths, `/`-separated,
//! the same spelling for the roots and the target): the kit looks nothing up.

use serde::{Deserialize, Serialize};

use super::path_text::is_within;

/// What a link's resolved target is, relative to the plugin and, when it has one, its marketplace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkReach {
    /// "Within the plugin's own directory".
    WithinPlugin,
    /// "Elsewhere within the same marketplace".
    WithinMarketplace,
    /// "Outside the marketplace".
    Outside,
}

/// What `link_reach` judges: the link's target and the two roots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkReachInput {
    /// The resolved target of the link.
    pub target: String,
    /// The plugin's root.
    pub plugin_root: String,
    /// The marketplace's root, when the plugin has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace_root: Option<String>,
}

/// Where a target resolves: "Within the plugin's own directory", "Elsewhere within the same
/// marketplace", or "Outside the marketplace". A plugin with no marketplace (a local path, a
/// `command` source in copy mode) has only its own directory to stay in.
#[must_use]
pub fn link_reach(input: &LinkReachInput) -> LinkReach {
    if is_within(&input.plugin_root, &input.target) {
        return LinkReach::WithinPlugin;
    }
    match &input.marketplace_root {
        Some(marketplace) if is_within(marketplace, &input.target) => LinkReach::WithinMarketplace,
        Some(_) | None => LinkReach::Outside,
    }
}

/// `rel` (a path below the plugin root) as a path from the folder `from_directory` (also below it).
#[must_use]
pub fn relative_from(from_directory: &str, rel: &str) -> String {
    let down: Vec<&str> = if from_directory.is_empty() || from_directory == "." {
        Vec::new()
    } else {
        from_directory.split('/').collect()
    };
    let to: Vec<&str> = rel.split('/').collect();
    let mut shared = 0;
    while shared < down.len() && shared < to.len() && down.get(shared) == to.get(shared) {
        shared += 1;
    }
    let up = down.iter().skip(shared).map(|_| "..");
    let path = up
        .chain(to.iter().skip(shared).copied())
        .collect::<Vec<&str>>()
        .join("/");
    if path.is_empty() {
        ".".to_owned()
    } else {
        path
    }
}

crate::registry::exports! {
    "format/link-rule" {
        "linkReach" => ok |input: LinkReachInput| link_reach(&input);
        "relativeFrom" => ok |from_directory: String, rel: String| relative_from(&from_directory, &rel);
    }
}
