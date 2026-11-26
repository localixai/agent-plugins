//! Port of `src/loading/plugin-folder.ts`, switched.
//!
//! What a folder passed as `--plugin-dir` is (plugins/create.md, "Load a folder of plugins"): "If the
//! folder has no `.claude-plugin/` directory and no plugin components at its top level, Claude Code
//! treats it as a folder of plugins", whose children that have a manifest load as plugins of their
//! own; the folder itself then loads nothing. A host loads a folder of plugins by classifying each
//! child, then calls `loadPlugin` on those that are plugins.

use serde::Serialize;

use crate::format::listing::list_sorted;
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::registry::Port;

/// Whether a folder is one plugin or holds several.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginFolderKind {
    /// One plugin.
    Plugin,
    /// A folder of plugins.
    FolderOfPlugins,
}

/// The top-level entries that mark a folder as one plugin: components, not metadata (oracle 2.1.319).
const MARKERS: [&str; 10] = [
    "skills",
    "commands",
    "agents",
    "hooks",
    "output-styles",
    "themes",
    "monitors",
    "workflows",
    "SKILL.md",
    ".lsp.json",
];

/// Judgment (spec/judgments.md, oracle 2.1.319): the docs name the rule, not its edges. A
/// `.claude-plugin/` folder marks a plugin unless it holds a `marketplace.json` and no `plugin.json`
/// (the docs' marketplace-beside-plugins case); `.mcp.json` marks one unless it is zero bytes;
/// `settings.json`, `bin/`, `CLAUDE.md` and a lower-case `skill.md` mark nothing.
pub async fn classify_plugin_folder(
    files: &dyn PluginFiles,
    signal: Option<&Signal>,
) -> PluginFolderKind {
    let marketplace = files
        .exists(".claude-plugin/marketplace.json", signal)
        .await;
    let manifest = files.exists(".claude-plugin/plugin.json", signal).await;
    let shelf = files.exists(".claude-plugin", signal).await;
    let mcp = files.stat(".mcp.json", signal).await;
    let mut marked = false;
    for marker in MARKERS {
        marked |= files.exists(marker, signal).await;
    }
    let shelf_marks = shelf && (manifest || !marketplace);
    if shelf_marks || mcp.is_some_and(|stat| stat.size > 0) || marked {
        PluginFolderKind::Plugin
    } else {
        PluginFolderKind::FolderOfPlugins
    }
}

/// A folder passed as `--plugin-dir` and what it holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDirectoryReading {
    /// The directory is a folder that holds plugins, not a plugin itself.
    pub is_folder: bool,
    /// The immediate subfolders that load, as separate plugins, in name order; none for a plugin.
    pub plugins: Vec<String>,
}

/// A folder passed as `--plugin-dir` and the plugins it loads: "Each immediate subfolder that has a
/// `.claude-plugin/plugin.json` manifest then loads as a separate plugin. Everything else in the
/// folder is skipped without an error, including a subfolder that has no manifest."
pub async fn read_plugin_directory(
    files: &dyn PluginFiles,
    signal: Option<&Signal>,
) -> PluginDirectoryReading {
    if classify_plugin_folder(files, signal).await == PluginFolderKind::Plugin {
        return PluginDirectoryReading {
            is_folder: false,
            plugins: Vec::new(),
        };
    }
    let mut plugins = Vec::new();
    for entry in list_sorted(files, ".", signal).await.unwrap_or_default() {
        if entry.is_directory
            && files
                .exists(
                    &format!("{}/.claude-plugin/plugin.json", entry.name),
                    signal,
                )
                .await
        {
            plugins.push(entry.name);
        }
    }
    PluginDirectoryReading {
        is_folder: true,
        plugins,
    }
}

crate::registry::exports! {
    "loading/plugin-folder" {
        "classifyPluginFolder" => async ok |files: Port, signal: Option<Signal>|
            classify_plugin_folder(&files, signal.as_ref()).await;
        "readPluginDirectory" => async ok |files: Port, signal: Option<Signal>|
            read_plugin_directory(&files, signal.as_ref()).await;
    }
}
