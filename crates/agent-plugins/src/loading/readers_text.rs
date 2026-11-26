//! Port of `src/loading/readers-text.ts`, switched.
//!
//! Reads the text of what discovery found for a kind that holds Markdown. This is the seam the
//! stream that owns the component's content replaces with its own parser (skill, command, agent,
//! output style); until then the text is kept as written.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::command_debug::ReaderContext;
use crate::format::frontmatter::parse_frontmatter;
use crate::format::issues::PluginIssue;
use crate::format::messages::missing_path;
use crate::js::{JsValue, promise_all, trim};
use crate::ports::BoxFuture;
use crate::ports::files::TextFileReader;
use crate::registry::Port;

/// A skill, command, agent or output style as loading finds it: its path, its name inside the plugin
/// and its text (`PluginTextFile` of `src/loading/plugin-model.ts`, a module of types only).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginTextFile {
    /// Relative to the plugin root; `None` for a command whose `content` is inline in the manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The name inside the plugin: `review`, `db:migrate`, `review:security`.
    pub name: String,
    /// The file's text, or the inline `content` of a manifest command.
    pub text: String,
    /// `<plugin>:<name>`: the name a user and the model see.
    pub qualified_name: String,
    /// A skill's folder, which also holds its supporting files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// A skill named by itself (the plugin root, or a manifest path to its folder): `name` is as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alone: Option<bool>,
    /// The manifest's object-map entry of a command, which carries its description, model and tools.
    #[serde(default, skip_serializing_if = "JsValue::is_undefined")]
    pub entry: JsValue,
}

/// What discovery finds of one file: its path and name, nothing parsed (`DiscoveredFile` of
/// `src/loading/discovered.ts`, a module of types only).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredFile {
    /// Relative to the plugin root; `None` for a command whose `content` is inline in the manifest.
    #[serde(default)]
    pub path: Option<String>,
    /// The name inside the plugin.
    pub name: String,
    /// A skill's folder, which also holds its supporting files.
    #[serde(default)]
    pub directory: Option<String>,
    /// A skill folder named by itself rather than found among the folders of a parent.
    #[serde(default)]
    pub alone: Option<bool>,
    /// A command declared in the manifest's object map.
    #[serde(default)]
    pub entry: JsValue,
}

fn qualify(plugin: &str, name: &str) -> String {
    format!("{plugin}:{name}")
}

/// A skill named by itself (the plugin root, or a manifest path to its folder): the frontmatter
/// `name` names it and the discovered name is the fallback. For the root, "`name` supplies the whole
/// final segment. Without a `name` field, Claude Code falls back to the plugin's directory name"
/// (skills.md, "How a skill gets its command name"); for a listed folder the oracle 2.1.319 does the same.
fn alone_skill_name(text: &str, fallback: &str) -> String {
    match parse_frontmatter(text).fields.get("name") {
        Some(JsValue::String(name)) if !trim(name).is_empty() => trim(name).to_owned(),
        _ => fallback.to_owned(),
    }
}

/// The text of one discovered file, or an inline command's `content`; `None` when it cannot be read.
async fn text_of(context: &ReaderContext<Port>, file: &DiscoveredFile) -> Option<String> {
    let Some(path) = &file.path else {
        return file
            .entry
            .get("content")
            .and_then(JsValue::as_str)
            .map(str::to_owned);
    };
    context.files.read_text(path, context.signal.as_ref()).await
}

/// One file read, or the issue that it is missing, pushed when its read settles (as the
/// TypeScript's `issues.push` after its `await`).
async fn read_one(
    kind: &str,
    context: &ReaderContext<Port>,
    file: &DiscoveredFile,
    issues: &Mutex<Vec<PluginIssue>>,
) -> Option<PluginTextFile> {
    let Some(text) = text_of(context, file).await else {
        let missing = missing_path(kind, file.path.as_deref().unwrap_or(&file.name));
        issues
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(missing);
        return None;
    };
    let name = if file.alone == Some(true) {
        alone_skill_name(&text, &file.name)
    } else {
        file.name.clone()
    };
    Some(PluginTextFile {
        path: file.path.clone(),
        qualified_name: qualify(&context.plugin_name, &name),
        name,
        text,
        directory: file.directory.clone(),
        alone: file.alone.filter(|alone| *alone),
        entry: file.entry.clone(),
    })
}

/// What a component reader returns: the value and what was wrong (`ComponentRead`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TextFilesRead {
    /// The files read.
    pub value: Vec<PluginTextFile>,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
}

/// Reads the text of the files discovery found for `kind` (`textFiles(kind, find)(context)`, with
/// `find(context)` already applied by the wrapper).
pub async fn text_files(
    kind: &str,
    found: &[DiscoveredFile],
    context: &ReaderContext<Port>,
) -> TextFilesRead {
    let issues = Mutex::new(Vec::new());
    let reads: Vec<BoxFuture<'_, Option<PluginTextFile>>> = found
        .iter()
        .map(|file| -> BoxFuture<'_, Option<PluginTextFile>> {
            Box::pin(read_one(kind, context, file, &issues))
        })
        .collect();
    let value = promise_all(reads).await.into_iter().flatten().collect();
    TextFilesRead {
        value,
        issues: issues
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    }
}

crate::registry::exports! {
    "loading/readers-text" {
        "textFiles" => async ok |kind: String, found: Vec<DiscoveredFile>, context: ReaderContext<Port>|
            text_files(&kind, &found, &context).await;
    }
}
