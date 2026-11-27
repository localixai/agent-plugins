//! Port of `src/loading/readers-config.ts`, switched.
//!
//! The component readers of the JSON configuration a plugin carries: hooks, LSP servers, monitors,
//! MCP servers and default settings. A file is parsed first and read by its kind's format reader;
//! an inline manifest value is read as is.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::discover_config::ConfigInput;
use super::merge_hooks::merge_hooks;
use super::readers_assets::ComponentRead;
use crate::format::hooks_settings::{
    HooksRead, HooksReadOptions, HooksSettings, read_hooks_file, read_hooks_settings,
};
use crate::format::issues::{PluginIssue, Severity, error, warning};
use crate::format::layout::{ComponentKind, declared_key};
use crate::format::lsp_config::{LspConfigRead, LspServer, merge_lsp_sources, read_lsp_config};
use crate::format::mcp_config::McpBundleConfig;
use crate::format::mcp_sources::{CollectOptions, collect_mcp_servers};
use crate::format::monitors::{Monitor, MonitorsRead, read_monitors};
use crate::format::plugin_settings::{
    PluginSettingsRead, has_applied_settings, read_plugin_settings,
};
use crate::format::read_json::{JsonRead, read_json, read_json_without_bom};
use crate::js::{JsObject, JsValue, promise_all};
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// What these readers read of discovery (`Discovered`): the configuration sources.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredConfig {
    /// The hooks sources.
    #[serde(default)]
    pub hooks: Vec<ConfigInput>,
    /// The LSP sources.
    #[serde(default)]
    pub lsp_servers: Vec<ConfigInput>,
    /// The monitors sources.
    #[serde(default)]
    pub monitors: Vec<ConfigInput>,
    /// The `settings.json` at the plugin root, when there is one.
    #[serde(default)]
    pub settings_file: Option<String>,
}

/// What these readers read of a component reader's context (`ReaderContext`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigContext {
    /// The plugin's files.
    pub files: Port,
    /// The manifest; `undefined` without one.
    #[serde(default)]
    pub manifest: JsValue,
    /// What discovery found.
    pub discovered: DiscoveredConfig,
    /// What contains MCP paths, and the host's MCP bundle capabilities.
    #[serde(default)]
    pub mcp: CollectOptions,
    /// Hook events a marketplace entry's inline hooks replace: a hook file's matchers for them are
    /// dropped.
    #[serde(default)]
    pub replaced_hook_events: Vec<String>,
    /// Aborts the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// A read that a source which does not parse leaves empty, with its issues.
trait SourceRead: Sized {
    fn failed(issues: Vec<PluginIssue>) -> Self;
}

impl SourceRead for HooksRead {
    fn failed(issues: Vec<PluginIssue>) -> Self {
        Self {
            hooks: HooksSettings::new(),
            issues,
            rejects_plugin: None,
        }
    }
}

impl SourceRead for LspConfigRead {
    fn failed(issues: Vec<PluginIssue>) -> Self {
        Self {
            servers: IndexMap::new(),
            issues,
        }
    }
}

impl SourceRead for MonitorsRead {
    fn failed(issues: Vec<PluginIssue>) -> Self {
        Self {
            monitors: Vec::new(),
            issues,
        }
    }
}

/// The reader of one kind: of a parsed file, of an inline value, and the wording of a parse failure.
struct InputReader<'a, R> {
    file: &'a (dyn Fn(&JsValue, &str) -> R + Sync),
    inline: &'a (dyn Fn(&JsValue) -> R + Sync),
    failure: &'a (dyn Fn(&str, &str) -> String + Sync),
    /// A byte order mark makes the file unreadable (hooks files only, spec J2.3).
    refuses_bom: bool,
}

/// Reads one source with the reader of its kind: a file is parsed first, an inline value is used as is.
async fn read_input<R: SourceRead>(
    context: &ConfigContext,
    input: &ConfigInput,
    reader: &InputReader<'_, R>,
) -> R {
    let file = match input {
        ConfigInput::Inline { inline } => return (reader.inline)(inline),
        ConfigInput::File { file, .. } => file,
    };
    let signal = context.signal.as_ref();
    let parsed = if reader.refuses_bom {
        read_json_without_bom(&context.files, file, signal).await
    } else {
        read_json(&context.files, file, signal).await
    };
    match parsed {
        JsonRead::Problem { problem, .. } => {
            R::failed(vec![error(file, &(reader.failure)(file, &problem))])
        }
        JsonRead::Value { value } => (reader.file)(&value, file),
    }
}

/// Every source of a kind, read together as the TypeScript's `Promise.all`.
async fn read_all<R: SourceRead + Send>(
    context: &ConfigContext,
    inputs: &[ConfigInput],
    reader: &InputReader<'_, R>,
) -> Vec<R> {
    let reads: Vec<BoxFuture<'_, R>> = inputs
        .iter()
        .map(|input| -> BoxFuture<'_, R> { Box::pin(read_input(context, input, reader)) })
        .collect();
    promise_all(reads).await
}

/// Plugin hooks drop an entry they cannot read and keep the rest, but for a guard (oracle 2.1.319).
const LENIENT: HooksReadOptions = HooksReadOptions {
    lenient: Some(true),
};

/// A hooks file that fails "does not load": its errors are reported as `Failed to load hooks from <path>: <reason>`.
fn hooks_file(raw: &JsValue, path: &str) -> HooksRead {
    let mut read = read_hooks_file(raw, LENIENT);
    for found in &mut read.issues {
        if found.severity == Severity::Error {
            *found = error(
                path,
                &format!("Failed to load hooks from {path}: {}", found.message),
            );
        }
    }
    read
}

/// The plugin's hooks, its hook files' matchers for the replaced events dropped.
pub async fn read_hooks(context: &ConfigContext) -> ComponentRead<HooksSettings> {
    let events = &context.replaced_hook_events;
    let file = |raw: &JsValue, path: &str| {
        let mut read = hooks_file(raw, path);
        read.hooks.retain(|event, _| !events.contains(event));
        read
    };
    let inline = |raw: &JsValue| read_hooks_settings(raw, LENIENT);
    let failure = |path: &str, reason: &str| format!("Failed to load hooks from {path}: {reason}");
    let reader = InputReader {
        file: &file,
        inline: &inline,
        failure: &failure,
        refuses_bom: true,
    };
    let reads = read_all(context, &context.discovered.hooks, &reader).await;
    let sources: Vec<HooksSettings> = reads.iter().map(|read| read.hooks.clone()).collect();
    ComponentRead {
        value: merge_hooks(&sources),
        issues: reads.iter().flat_map(|read| read.issues.clone()).collect(),
        rejects_plugin: reads
            .iter()
            .any(|read| read.rejects_plugin == Some(true))
            .then_some(true),
    }
}

/// The plugin's LSP servers, every source joined.
pub async fn read_lsp_servers(
    context: &ConfigContext,
) -> ComponentRead<IndexMap<String, LspServer>> {
    let file = |raw: &JsValue, path: &str| read_lsp_config(raw, path);
    let inline = |raw: &JsValue| read_lsp_config(raw, "lspServers");
    let failure =
        |path: &str, reason: &str| format!("Invalid LSP server config for \"{path}\": {reason}");
    let reader = InputReader {
        file: &file,
        inline: &inline,
        failure: &failure,
        refuses_bom: false,
    };
    let reads = read_all(context, &context.discovered.lsp_servers, &reader).await;
    let merged = merge_lsp_sources(&reads);
    ComponentRead {
        value: merged.servers,
        issues: merged.issues,
        rejects_plugin: None,
    }
}

/// The plugin's monitors, in source order.
pub async fn read_monitor_list(context: &ConfigContext) -> ComponentRead<Vec<Monitor>> {
    let origin = declared_key(&context.manifest, ComponentKind::Monitors)
        .map_or("monitors", |declared| declared.key);
    let file = |raw: &JsValue, path: &str| read_monitors(raw, path);
    let inline = |raw: &JsValue| read_monitors(raw, origin);
    let failure = |path: &str, reason: &str| format!("Invalid monitors in {path}: {reason}");
    let reader = InputReader {
        file: &file,
        inline: &inline,
        failure: &failure,
        refuses_bom: false,
    };
    let reads = read_all(context, &context.discovered.monitors, &reader).await;
    ComponentRead {
        value: reads
            .iter()
            .flat_map(|read| read.monitors.clone())
            .collect(),
        issues: reads.iter().flat_map(|read| read.issues.clone()).collect(),
        rejects_plugin: None,
    }
}

/// The servers, and the bundled ones' own settings (`Plugin.mcpBundles`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct McpServersValue {
    /// The servers, by name.
    pub servers: JsObject,
    /// The bundled servers' settings, by name.
    pub bundles: IndexMap<String, McpBundleConfig>,
}

/// The plugin's MCP servers: the root `.mcp.json`, then each source the manifest declares.
pub async fn read_mcp_servers(context: &ConfigContext) -> ComponentRead<McpServersValue> {
    let declared = context
        .manifest
        .get("mcpServers")
        .cloned()
        .unwrap_or_default();
    let collected = collect_mcp_servers(context.files, &declared, context.mcp.clone()).await;
    ComponentRead {
        value: McpServersValue {
            servers: collected.servers,
            bundles: collected.bundles,
        },
        issues: collected.issues,
        rejects_plugin: collected.rejects_plugin,
    }
}

/// `settings.json` as read, with the issue its parse failure raised.
async fn read_settings_file(
    context: &ConfigContext,
) -> (Option<PluginSettingsRead>, Vec<PluginIssue>) {
    let Some(file) = &context.discovered.settings_file else {
        return (None, Vec::new());
    };
    match read_json(&context.files, file, context.signal.as_ref()).await {
        // A load warning, never a failure (spec §12.6 item 1, J12.10).
        JsonRead::Problem { problem, .. } => (
            None,
            vec![warning(
                file,
                &format!("settings.json is not valid JSON: {problem}"),
            )],
        ),
        JsonRead::Value { value } => (Some(read_plugin_settings(&value, file)), Vec::new()),
    }
}

/// `settings.json` over the manifest's `settings` key: "when both exist and `settings.json` sets at
/// least one supported key, `settings.json` applies and the manifest's `settings` is ignored"
/// (components.md, "Default settings").
pub async fn read_settings(context: &ConfigContext) -> ComponentRead<JsObject> {
    let (read, mut issues) = read_settings_file(context).await;
    let file_wins = read.as_ref().is_some_and(has_applied_settings);
    let manifest_read = if file_wins {
        None
    } else {
        context
            .manifest
            .get("settings")
            .filter(|inline| !inline.is_undefined())
            .map(|inline| read_plugin_settings(inline, "settings"))
    };
    issues.extend(read.iter().flat_map(|one| one.issues.clone()));
    issues.extend(manifest_read.iter().flat_map(|one| one.issues.clone()));
    let chosen = if file_wins {
        read
    } else {
        manifest_read.or(read)
    };
    ComponentRead {
        value: chosen.map(|one| one.settings).unwrap_or_default(),
        issues,
        rejects_plugin: None,
    }
}

crate::registry::exports! {
    "loading/readers-config" {
        "readHooks" => async ok |context: ConfigContext| read_hooks(&context).await;
        "readLspServers" => async ok |context: ConfigContext| read_lsp_servers(&context).await;
        "readMonitorList" => async ok |context: ConfigContext| read_monitor_list(&context).await;
        "readMcpServers" => async ok |context: ConfigContext| read_mcp_servers(&context).await;
        "readSettings" => async ok |context: ConfigContext| read_settings(&context).await;
    }
}
