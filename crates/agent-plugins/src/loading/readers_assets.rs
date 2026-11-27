//! Port of `src/loading/readers-assets.ts`, switched.
//!
//! Themes and workflows (plugins/components.md, "Themes and output styles"; workflows.md,
//! "Distribute a workflow in a plugin"): each file is read once, the theme through `readTheme` and
//! the workflow's `meta` through `inspectWorkflowScript`, which never runs the script. What is wrong
//! with a file comes back as issues at the file's path; the component stays in the plugin either
//! way, so a host sees the file and the reason (a theme that is not a JSON object has no `theme`; a
//! script without a readable `meta` has no `meta` and no qualified name).

use serde::{Deserialize, Serialize};

use super::readers_text::DiscoveredFile;
use crate::format::issues::PluginIssue;
use crate::format::messages::missing_path;
use crate::format::theme::{Theme, read_theme};
use crate::format::workflow_meta::WorkflowMeta;
use crate::format::workflow_script::inspect_workflow_script;
use crate::js::promise_all;
use crate::ports::files::TextFileReader;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// What these readers read of discovery (`Discovered`): the themes and the workflows.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DiscoveredAssets {
    /// The theme files.
    #[serde(default)]
    pub themes: Vec<DiscoveredFile>,
    /// The workflow scripts.
    #[serde(default)]
    pub workflows: Vec<DiscoveredFile>,
}

/// What these readers read of a component reader's context (`ReaderContext`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetContext {
    /// The plugin's files.
    pub files: Port,
    /// The plugin's name.
    pub plugin_name: String,
    /// What discovery found.
    pub discovered: DiscoveredAssets,
    /// Aborts the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// A theme of the plugin (`PluginTheme` of `src/loading/plugin-model.ts`, a module of types only).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PluginTheme {
    /// Relative to the plugin root.
    pub path: String,
    /// The slug: the file name without `.json`.
    pub name: String,
    /// The plugin it comes from: `/theme` marks the theme as from it.
    pub plugin: String,
    /// The theme, when the file is a JSON object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
}

/// A workflow script of the plugin, its `meta` block read without running it (workflows.md,
/// "Distribute a workflow in a plugin"). It runs as `/<plugin>:<meta.name>`: `qualified_name` is
/// `None` when `meta` cannot be read, since Claude Code then drops it from `/` autocomplete
/// (`PluginWorkflow` of `src/loading/plugin-model.ts`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginWorkflow {
    /// Relative to the plugin root.
    pub path: String,
    /// The file name without `.js`.
    pub name: String,
    /// `<plugin>:<meta.name>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    /// The `meta`, when it reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<WorkflowMeta>,
}

/// What a component reader gives (`ComponentRead`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentRead<T> {
    /// The component's values.
    pub value: T,
    /// What is wrong with them.
    pub issues: Vec<PluginIssue>,
    /// Reading this component failed in a way that keeps the whole plugin from loading (oracle 2.1.319).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejects_plugin: Option<bool>,
}

struct Source<'a> {
    file: &'a DiscoveredFile,
    path: &'a str,
    text: Option<String>,
}

async fn sources_of<'a>(context: &'a AssetContext, found: &'a [DiscoveredFile]) -> Vec<Source<'a>> {
    promise_all(
        found
            .iter()
            .filter_map(|file| {
                let path = file.path.as_deref()?;
                let read: BoxFuture<'a, Source<'a>> = Box::pin(async move {
                    let text = context.files.read_text(path, context.signal.as_ref()).await;
                    Source { file, path, text }
                });
                Some(read)
            })
            .collect(),
    )
    .await
}

fn where_at(path: &str, issues: Vec<PluginIssue>) -> impl Iterator<Item = PluginIssue> + '_ {
    issues.into_iter().map(move |issue| PluginIssue {
        path: path.to_owned(),
        ..issue
    })
}

fn missing(kind: &str, path: &str) -> PluginIssue {
    missing_path(kind, path)
}

/// The plugin's themes.
pub async fn read_themes(context: &AssetContext) -> ComponentRead<Vec<PluginTheme>> {
    let sources = sources_of(context, &context.discovered.themes).await;
    let mut value = Vec::new();
    let mut issues = Vec::new();
    for Source { file, path, text } in sources {
        let Some(text) = text else {
            issues.push(missing("themes", path));
            continue;
        };
        let read = read_theme(&file.name, &text);
        issues.extend(where_at(path, read.issues));
        value.push(PluginTheme {
            path: path.to_owned(),
            name: file.name.clone(),
            plugin: context.plugin_name.clone(),
            theme: read.theme,
        });
    }
    ComponentRead {
        value,
        issues,
        rejects_plugin: None,
    }
}

/// The plugin's workflows.
pub async fn read_workflows(context: &AssetContext) -> ComponentRead<Vec<PluginWorkflow>> {
    let sources = sources_of(context, &context.discovered.workflows).await;
    let mut value = Vec::new();
    let mut issues = Vec::new();
    for Source { file, path, text } in sources {
        let Some(text) = text else {
            issues.push(missing("workflows", path));
            continue;
        };
        let report = inspect_workflow_script(&text);
        issues.extend(where_at(path, report.issues));
        let qualified_name = report
            .meta
            .as_ref()
            .map(|meta| format!("{}:{}", context.plugin_name, meta.name));
        value.push(PluginWorkflow {
            path: path.to_owned(),
            name: file.name.clone(),
            qualified_name,
            meta: report.meta,
        });
    }
    ComponentRead {
        value,
        issues,
        rejects_plugin: None,
    }
}

crate::registry::exports! {
    "loading/readers-assets" {
        "readThemes" => async ok |context: AssetContext| read_themes(&context).await;
        "readWorkflows" => async ok |context: AssetContext| read_workflows(&context).await;
    }
}
