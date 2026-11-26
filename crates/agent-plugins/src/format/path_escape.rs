//! Port of `src/format/path-escape.ts`, switched.
//!
//! A component path that leaves the plugin (plugins/loading.md, "Paths that escape the plugin
//! directory"): "Whether a plugin loads in place or from a cached copy, Claude Code doesn't let it
//! declare components outside its own directory", whether the path is declared in `plugin.json` or
//! in a marketplace entry. Rejected: a path that points outside as written, a symlink that leads
//! outside the plugin "other than links between plugins within one marketplace", and on macOS and
//! Linux a path with a backslash. The plugin "loads without that component". The loader (`locate`)
//! decides every declared path through `escapedPathIssue`.

use serde::Deserialize;

use super::issues::{PluginIssue, error};
use super::link_rule::{LinkReach, LinkReachInput, link_reach};
use super::messages::{BACKSLASH_TAIL, path_escapes};
use super::path_text::{
    PathPlatform, escapes_as_written, leaves_folder, refused_backslash, resolve_from,
};

/// The real paths (`/`-separated, one spelling) a link is judged against.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkRoots {
    /// The plugin's own folder.
    pub plugin_root: String,
    /// The marketplace the plugin is in, when it is in one; links to other plugins of it are allowed.
    #[serde(default)]
    pub marketplace_root: Option<String>,
}

/// A path a plugin declared.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredPath {
    /// The field that declared the path: `commands`, `hooks`...
    pub component: String,
    /// The path as written.
    pub written: String,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// Where the path really leads when it is, or crosses, a symlink: an absolute path, or one
    /// relative to the plugin folder; absent when it is not.
    #[serde(default)]
    pub resolves_to: Option<String>,
    /// The folders to judge a link by. Without them nothing but the plugin's own folder is known, so
    /// a link target that is absolute or climbs out of the folder leaves the plugin (judgment).
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// The path is one of the plugin's own files (`hooks/hooks.json`, `monitors/monitors.json`,
    /// `settings.json`, `SKILL.md` at the root): errors.md, "Path escapes plugin directory", reports it
    /// by its full path, and in a local marketplace says it is outside that marketplace too.
    #[serde(default)]
    pub own: Option<bool>,
}

/// Whether the real path a link leads to is outside the plugin, and outside its marketplace's other plugins.
fn link_leaves(resolves_to: &str, platform: PathPlatform, roots: Option<&LinkRoots>) -> bool {
    let Some(roots) = roots else {
        return leaves_folder(resolves_to, platform);
    };
    let target = resolve_from(&roots.plugin_root, resolves_to, platform);
    link_reach(&LinkReachInput {
        target,
        plugin_root: roots.plugin_root.clone(),
        marketplace_root: roots.marketplace_root.clone(),
    }) == LinkReach::Outside
}

/// A path refused as written; the backslash tail when only its backslash refuses it (the same path
/// with the backslashes read as names stays inside).
fn as_written_issue(path: &DeclaredPath) -> PluginIssue {
    let only_backslash = refused_backslash(&path.written, path.platform)
        && !escapes_as_written(&path.written.replace('\\', "_"), path.platform);
    let tail = if only_backslash { BACKSLASH_TAIL } else { "" };
    error(
        &path.component,
        &format!(
            "{}{tail}",
            path_escapes(&path.component, &path.written, None, false)
        ),
    )
}

/// The issue that drops the path, or `None` when it stays inside the plugin.
#[must_use]
pub fn escaped_path_issue(path: &DeclaredPath) -> Option<PluginIssue> {
    if escapes_as_written(&path.written, path.platform) {
        return Some(as_written_issue(path));
    }
    let resolves_to = path.resolves_to.as_deref()?;
    if !link_leaves(resolves_to, path.platform, path.roots.as_ref()) {
        return None;
    }
    let own_roots = path.roots.as_ref().filter(|_| path.own == Some(true));
    let shown = match own_roots {
        Some(roots) => format!("{}/{}", roots.plugin_root, path.written),
        None => path.written.clone(),
    };
    let in_marketplace = own_roots.is_some_and(|roots| roots.marketplace_root.is_some());
    Some(error(
        &path.component,
        &path_escapes(&path.component, &shown, Some(resolves_to), in_marketplace),
    ))
}

crate::registry::exports! {
    "format/path-escape" {
        "escapedPathIssue" => ok |path: DeclaredPath| escaped_path_issue(&path);
    }
}
