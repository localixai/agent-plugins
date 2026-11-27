//! Port of `src/loading/command-debug.ts`, switched.
//!
//! The debug-log line for a manifest `commands` directory that holds no command.

use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use crate::format::load_files::Files;
use crate::format::path_text::is_within;
use crate::js::{JsValue, Pattern, promise_all};
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};
use crate::registry::catch_host;

/// `/^\.\/+/`.
static LEADING_DOT_SLASH: Pattern = LazyLock::new(|| Regex::new(r"^\./+").ok());
/// `/\/+$/`.
static TRAILING_SLASHES: Pattern = LazyLock::new(|| Regex::new(r"/+$").ok());

/// `text.replace(pattern, '')`.
fn without(pattern: &Pattern, text: &str) -> String {
    pattern.as_ref().map_or_else(
        || text.to_owned(),
        |regex| regex.replace(text, "").into_owned(),
    )
}

fn canonical(path: &str) -> String {
    without(
        &TRAILING_SLASHES,
        &without(&LEADING_DOT_SLASH, &path.replace('\\', "/")),
    )
}

fn inside(path: &str, directory: &str) -> bool {
    directory.is_empty() || directory == "." || is_within(directory, &canonical(path))
}

/// What the line reads of a component reader's context (`ReaderContext`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderContext<F> {
    /// The plugin's files.
    pub files: F,
    /// The manifest, when there is one: its `commands`.
    #[serde(default)]
    pub manifest: Option<JsValue>,
    /// The plugin's name.
    pub plugin_name: String,
    /// Aborts the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// What the line reads of a command file (`PluginTextFile`): its path.
#[derive(Clone, Debug, Deserialize)]
pub struct CommandFile {
    /// The file's path, as the loader found it.
    #[serde(default)]
    pub path: Option<String>,
}

/// troubleshooting.md: this diagnostic belongs in the debug log, not the Plugin Errors tab, with the
/// directory and the advice after the colon. The path is the manifest's, as written: Claude Code
/// prints the absolute one, which is the host's (`Locations.pluginRoot`), not the loader's.
///
/// A `commands` list item that is no text is passed over (the TypeScript throws a `TypeError`
/// there; the manifest's validation never lets one through). The paths are checked together, as
/// `Promise.all` did; a `stat` the host throws for is no directory (`.catch(() => undefined)`).
pub async fn command_directory_debug<F: PluginFiles>(
    context: &ReaderContext<F>,
    commands: &[CommandFile],
) -> Vec<String> {
    let value = context
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.get("commands"));
    let paths: Vec<&str> = match value {
        Some(JsValue::String(path)) => vec![path.as_str()],
        Some(JsValue::Array(paths)) => paths.iter().filter_map(JsValue::as_str).collect(),
        _ => Vec::new(),
    };
    let checks: Vec<BoxFuture<'_, Vec<String>>> = paths
        .into_iter()
        .map(|path| -> BoxFuture<'_, Vec<String>> {
            Box::pin(async move {
                let directory = canonical(path);
                let stat = catch_host(context.files.stat(&directory, context.signal.as_ref()))
                    .await
                    .ok()
                    .flatten();
                let empty = !commands.iter().any(|command| {
                    command
                        .path
                        .as_deref()
                        .is_some_and(|path| inside(path, &directory))
                });
                if stat.is_some_and(|stat| stat.is_directory) && empty {
                    vec![format!(
                        "Warning: No commands found in plugin {} custom directory: {path}. Expected .md files or SKILL.md in subdirectories.",
                        context.plugin_name
                    )]
                } else {
                    Vec::new()
                }
            })
        })
        .collect();
    promise_all(checks).await.into_iter().flatten().collect()
}

crate::registry::exports! {
    "loading/command-debug" {
        "commandDirectoryDebug" => async ok |context: ReaderContext<Files>, commands: Vec<CommandFile>|
            command_directory_debug(&context, &commands).await;
    }
}
