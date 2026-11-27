//! Ported from `src/loading/validate/plugin-contents.ts`; only the core uses it now, so it has no
//! wrapper.
//!
//! Everything beside the manifest that the validator reads in a plugin folder: a root `CLAUDE.md`,
//! the Markdown components, the hooks files and the MCP servers files.

use indexmap::IndexSet;

use crate::format::validate::finding::{Finding, ValidationIssue, in_file, warning_at};
use crate::format::validate::plugin_manifest::declared_options;
use crate::js::promise_all;
use crate::js::{JsObject, JsValue};
use crate::loading::validate::components::{component_issues, manifest_paths};
use crate::loading::validate::json_files::{hooks_file_issues, mcp_file_issues};
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};

const CLAUDE_MD: &str = "CLAUDE.md at the plugin root is not loaded as project context. To ship context with your plugin, use a skill (skills/<name>/SKILL.md) instead.";

/// `/^https?:\/\//i`: a remote URL, not a file of the plugin.
fn remote(path: &str) -> bool {
    let starts = |prefix: &str| {
        path.get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
    };
    starts("http://") || starts("https://")
}

/// The paths with each one kept once, in first order (`[...new Set(paths)]`).
fn distinct(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .collect::<IndexSet<_>>()
        .into_iter()
        .collect()
}

fn member<'a>(manifest: &'a JsObject, key: &str) -> &'a JsValue {
    manifest.get(key).unwrap_or(&JsValue::Undefined)
}

async fn hook_issues(
    files: &dyn PluginFiles,
    manifest: &JsObject,
    signal: Option<&Signal>,
) -> Vec<ValidationIssue> {
    let paths: Vec<String> = std::iter::once("hooks/hooks.json".to_owned())
        .chain(manifest_paths(member(manifest, "hooks")))
        .collect();
    let paths = distinct(paths);
    let reads: Vec<BoxFuture<'_, Vec<ValidationIssue>>> = paths
        .iter()
        .map(|path| -> BoxFuture<'_, _> { Box::pin(hooks_file_issues(files, path, signal)) })
        .collect();
    promise_all(reads).await.concat()
}

async fn mcp_issues(
    files: &dyn PluginFiles,
    manifest: &JsObject,
    signal: Option<&Signal>,
) -> Vec<ValidationIssue> {
    let declared = declared_options(manifest);
    let paths: Vec<String> = std::iter::once(".mcp.json".to_owned())
        .chain(
            manifest_paths(member(manifest, "mcpServers"))
                .into_iter()
                .filter(|path| !remote(path)),
        )
        .collect();
    let paths = distinct(paths);
    let reads: Vec<BoxFuture<'_, Vec<ValidationIssue>>> = paths
        .iter()
        .map(|path| -> BoxFuture<'_, _> {
            Box::pin(mcp_file_issues(files, path, &declared, signal))
        })
        .collect();
    promise_all(reads).await.concat()
}

/// The plugin's own manifest and what is written inline in it.
pub struct PluginManifestSite<'a> {
    /// The parsed manifest.
    pub manifest: &'a JsObject,
    /// The manifest's file.
    pub file: &'a str,
    /// What its inline `hooks` and `mcpServers` hold.
    pub inline: Vec<Finding>,
}

/// Everything beside the manifest that the validator reads in a plugin folder.
pub async fn plugin_contents(
    files: &dyn PluginFiles,
    site: &PluginManifestSite<'_>,
    signal: Option<&Signal>,
) -> Vec<ValidationIssue> {
    let manifest = site.manifest;
    let own = files.stat("CLAUDE.md", signal).await;
    let claude = if own.is_some_and(|own| own.is_file && !own.is_symlink) {
        in_file("CLAUDE.md", &[warning_at("root", CLAUDE_MD)], "")
    } else {
        Vec::new()
    };
    let mut issues = claude;
    issues.extend(component_issues(files, signal).await);
    issues.extend(hook_issues(files, manifest, signal).await);
    issues.extend(in_file(site.file, &site.inline, ""));
    issues.extend(mcp_issues(files, manifest, signal).await);
    issues
}
