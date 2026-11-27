//! Port of `src/loading/load-plugin.ts`, switched.
//!
//! Loading a plugin folder into a typed `Plugin`: one crossing per load, the host's files a port the
//! core asks through one load's view of it (`format/load_files.rs`).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::channels::{ChannelManifest, channel_issues};
use super::command_debug::{CommandFile, command_directory_debug};
use super::component_readers::{COMPONENT_READERS, ReaderContext};
use super::discover::{DiscoverOptions, discover_components};
use super::manifest_gate::{GateOptions, manifest_gate};
use super::readers_assets::{PluginTheme, PluginWorkflow};
use super::readers_text::PluginTextFile;
use crate::format::hooks_settings::HooksSettings;
use crate::format::issues::{PluginIssue, error};
use crate::format::layout::{ComponentKind, STANDARD_LAYOUT};
use crate::format::load_files::{Files, load_files};
use crate::format::locate::LocateOptions;
use crate::format::lsp_config::LspServer;
use crate::format::manifest::read_manifest;
use crate::format::marketplace::entry_merge::merge_entry_with_manifest;
use crate::format::marketplace::types::MarketplaceEntry;
use crate::format::mcp_config::McpBundleConfig;
use crate::format::mcp_sources::CollectOptions;
use crate::format::monitors::Monitor;
use crate::format::names::name_load_issues;
use crate::format::path_escape::LinkRoots;
use crate::format::path_text::PathPlatform;
use crate::format::read_json::{JsonRead, read_json};
use crate::js::{JsObject, JsValue, promise_all_pair};
use crate::ports::Signal;
use crate::ports::files::PluginFiles;
use crate::registry::Port;

/// What a load is given (`LoadPluginInput`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPluginInput {
    /// Read access to the plugin folder: its root is the plugin root.
    pub files: Files,
    /// The name to load a plugin without a manifest under: "the plugin name then comes from the
    /// marketplace entry, or from the directory name when you load the plugin with `--plugin-dir`"
    /// (manifest-reference.md, "Manifest file"). The manifest's `name` wins when there is one.
    #[serde(default)]
    pub fallback_name: Option<String>,
    /// Opens `.mcpb` and `.dxt` bundles named by `mcpServers`; without it a bundle is a warning.
    #[serde(default)]
    pub bundles: Option<Port>,
    /// The platform the host runs on; a backslash in a declared path escapes the plugin on `posix`.
    #[serde(default)]
    pub platform: Option<PathPlatform>,
    /// Where the plugin and its marketplace really are, so a link to another plugin of the
    /// marketplace loads.
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// The marketplace entry the plugin is installed from (`readMarketplace`), its `strict`
    /// included: the plugin loads with the manifest `mergeEntryWithManifest` gives
    /// (plugins/manifest-reference.md, "How entry fields combine with `plugin.json`"). Its name
    /// loads a plugin without `plugin.json`, ahead of `fallbackName`.
    #[serde(default)]
    pub entry: Option<MarketplaceEntry>,
    /// Aborts the load.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The manifest step (`ManifestStep`).
#[derive(Clone, Debug, Serialize)]
pub struct ManifestStep {
    /// Whether `.claude-plugin/plugin.json` exists.
    pub present: bool,
    /// The manifest that loaded; `undefined` without one.
    pub manifest: JsValue,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
}

/// Reads the optional manifest: absent is fine, unreadable JSON or a manifest with an error is not.
pub async fn load_manifest(files: &dyn PluginFiles, signal: Option<&Signal>) -> ManifestStep {
    let location = STANDARD_LAYOUT.of(ComponentKind::Manifest).location;
    if !files.exists(location, signal).await {
        return ManifestStep {
            present: false,
            manifest: JsValue::Undefined,
            issues: Vec::new(),
        };
    }
    let raw = match read_json(files, location, signal).await {
        JsonRead::Problem { problem, .. } => {
            let message = format!("{location} could not be read as JSON: {problem}");
            return ManifestStep {
                present: true,
                manifest: JsValue::Undefined,
                issues: vec![error(location, &message)],
            };
        }
        JsonRead::Value { value } => value,
    };
    let read = read_manifest(&raw);
    ManifestStep {
        present: true,
        manifest: read.manifest,
        issues: read.issues,
    }
}

/// The manifest the plugin loads with: its own, or, from a marketplace entry, the merge of the two.
struct MergedStep {
    manifest: JsValue,
    issues: Vec<PluginIssue>,
    scan_default_skills: bool,
    replaced_hook_events: Vec<String>,
}

/// The manifest step of a load, or why the plugin does not load: a `plugin.json` that fails, or an
/// entry whose merge with it fails (the strict conflict).
async fn manifest_for(
    files: &Files,
    entry: Option<&MarketplaceEntry>,
    signal: Option<&Signal>,
) -> Result<MergedStep, Vec<PluginIssue>> {
    let own = load_manifest(files, signal).await;
    if own.present && own.manifest.is_undefined() {
        return Err(own.issues);
    }
    let step = with_entry(own, entry);
    if step.manifest.is_undefined() && entry.is_some() {
        return Err(step.issues);
    }
    Ok(step)
}

fn with_entry(read: ManifestStep, entry: Option<&MarketplaceEntry>) -> MergedStep {
    let Some(entry) = entry else {
        return MergedStep {
            manifest: read.manifest,
            issues: read.issues,
            scan_default_skills: true,
            replaced_hook_events: Vec::new(),
        };
    };
    let merge = merge_entry_with_manifest(entry, read.manifest.as_object());
    let mut issues = read.issues;
    issues.extend(merge.issues);
    MergedStep {
        manifest: merge.manifest.map_or(JsValue::Undefined, JsValue::Object),
        issues,
        scan_default_skills: merge.scan_default_skills,
        replaced_hook_events: merge.replaced_hook_events,
    }
}

const NO_NAME: &str = "The plugin has no manifest and no name to load it under: pass the marketplace entry name or the directory name";

/// The manifest's name, else the fallback; a manifest-less plugin with neither does not load
/// (judgment).
fn name_of(manifest: &JsValue, fallback_name: Option<&str>) -> (Option<String>, Vec<PluginIssue>) {
    if !manifest.is_undefined() {
        let name = manifest
            .get("name")
            .and_then(JsValue::as_str)
            .unwrap_or_default();
        return (Some(name.to_owned()), Vec::new());
    }
    let Some(fallback_name) = fallback_name else {
        return (None, vec![error("name", NO_NAME)]);
    };
    let issues = name_load_issues(fallback_name);
    let name = issues.is_empty().then(|| fallback_name.to_owned());
    (name, issues)
}

/// A plugin that loaded (`Plugin` of `src/loading/plugin-model.ts`, a module of types only), its
/// members in the order the TypeScript writes them.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    /// The name components are namespaced under.
    pub name: String,
    /// The manifest; `undefined` without one.
    pub manifest: JsValue,
    /// The skill files.
    pub skills: Vec<PluginTextFile>,
    /// The command files.
    pub commands: Vec<PluginTextFile>,
    /// The agent files.
    pub agents: Vec<PluginTextFile>,
    /// The hooks.
    pub hooks: HooksSettings,
    /// The MCP servers, by name.
    pub mcp_servers: JsObject,
    /// The LSP servers, by name.
    pub lsp_servers: IndexMap<String, LspServer>,
    /// The output style files.
    pub output_styles: Vec<PluginTextFile>,
    /// The workflows.
    pub workflows: Vec<PluginWorkflow>,
    /// The themes.
    pub themes: Vec<PluginTheme>,
    /// The monitors.
    pub monitors: Vec<Monitor>,
    /// The files directly in `bin/`.
    pub bin: Vec<String>,
    /// The plugin's `settings.json`.
    pub settings: JsObject,
    /// The bundled servers' own settings, by name.
    pub mcp_bundles: IndexMap<String, McpBundleConfig>,
    /// The manifest's `userConfig`, or none.
    pub user_config: JsValue,
    /// Whether the plugin has `bin/`.
    pub has_bin: bool,
}

/// What a load gives (`LoadedPlugin`).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPlugin {
    /// The plugin; none when it does not load at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<Plugin>,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
    /// The debug log's lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<Vec<String>>,
}

/// A load that gives no plugin.
const fn not_loaded(issues: Vec<PluginIssue>) -> LoadedPlugin {
    LoadedPlugin {
        plugin: None,
        issues,
        debug: None,
    }
}

/// What decides whether a declared path leaves the plugin, from the input.
fn discover_options_of(input: &LoadPluginInput) -> LocateOptions {
    LocateOptions {
        platform: input.platform,
        roots: input.roots.clone(),
        signal: input.signal.clone(),
    }
}

/// MCP paths are contained by the same rule as every other component path.
fn mcp_options(input: &LoadPluginInput) -> CollectOptions {
    CollectOptions {
        platform: input.platform,
        roots: input.roots.clone(),
        signal: input.signal.clone(),
        bundles: input.bundles,
    }
}

/// Loads a plugin folder into a typed `Plugin` (docs/API.md, "Loading"): the optional manifest,
/// every component in the standard layout and the manifest's paths, each reader's issues
/// collected. Never throws. The plugin is `undefined` only when it does not load at all: a plugin
/// folder or manifest that cannot be checked or leads outside (`manifestGate`), a manifest that is
/// not valid JSON or fails validation, no name to load it under, a guard hook that cannot be read
/// or an inline MCP server of the manifest that cannot be read (oracle 2.1.319,
/// spec/judgments.md). Any other problem (a path that is missing or escapes the plugin, a file
/// Claude Code would skip) is an issue and skips that component, as the docs say for paths ("the
/// plugin loads without that component"); the rest loads (spec/judgments.md).
pub async fn load_plugin(input: LoadPluginInput) -> LoadedPlugin {
    let signal = input.signal.clone();
    let files = load_files(input.files.clone(), signal.clone());
    let located = discover_options_of(&input);
    let gate_options = GateOptions {
        roots: located.roots.clone(),
        platform: located.platform,
        signal: located.signal.clone(),
    };
    if let Some(gate) = manifest_gate(&files, &gate_options).await {
        return not_loaded(vec![gate]);
    }
    let step = match manifest_for(&files, input.entry.as_ref(), signal.as_ref()).await {
        Ok(step) => step,
        Err(failed) => return not_loaded(failed),
    };
    let (name, named_issues) = name_of(&step.manifest, input.fallback_name.as_deref());
    let mut issues = step.issues;
    issues.extend(named_issues);
    let Some(name) = name else {
        return not_loaded(issues);
    };
    let options = DiscoverOptions {
        locate: located,
        scan_default_skills: Some(step.scan_default_skills),
    };
    let discovered =
        discover_components(files.clone(), step.manifest.clone(), &name, options).await;
    let context = ReaderContext {
        files,
        manifest: step.manifest,
        plugin_name: name,
        discovered,
        mcp: mcp_options(&input),
        replaced_hook_events: step.replaced_hook_events,
        signal,
    };
    read_plugin(context, issues).await
}

/// Runs every reader over what discovery found, then builds the plugin of what they read.
async fn read_plugin(context: ReaderContext, mut issues: Vec<PluginIssue>) -> LoadedPlugin {
    let run = &COMPONENT_READERS;
    let first = promise_all_pair(
        promise_all_pair((run.skills)(&context), (run.commands)(&context)),
        promise_all_pair(
            (run.agents)(&context),
            promise_all_pair(
                (run.hooks)(&context),
                promise_all_pair((run.mcp_servers)(&context), (run.lsp_servers)(&context)),
            ),
        ),
    );
    let ((skills, commands), (agents, (hooks, (mcp_servers, lsp_servers)))) = first.await;
    let second = promise_all_pair(
        promise_all_pair((run.output_styles)(&context), (run.workflows)(&context)),
        promise_all_pair(
            (run.themes)(&context),
            promise_all_pair(
                (run.monitors)(&context),
                promise_all_pair((run.bin)(&context), (run.settings)(&context)),
            ),
        ),
    );
    let ((output_styles, workflows), (themes, (monitors, (bin, settings)))) = second.await;
    // A component failed so that the plugin does not load (oracle 2.1.319).
    let rejected = [hooks.rejects_plugin, mcp_servers.rejects_plugin].contains(&Some(true));
    issues.extend(context.discovered.issues.iter().cloned());
    for read in [
        &skills.issues,
        &commands.issues,
        &agents.issues,
        &hooks.issues,
    ] {
        issues.extend(read.iter().cloned());
    }
    for read in [
        &mcp_servers.issues,
        &lsp_servers.issues,
        &output_styles.issues,
    ] {
        issues.extend(read.iter().cloned());
    }
    for read in [
        &workflows.issues,
        &themes.issues,
        &monitors.issues,
        &bin.issues,
    ] {
        issues.extend(read.iter().cloned());
    }
    issues.extend(settings.issues);
    if rejected {
        return not_loaded(issues);
    }
    let manifest = &context.manifest;
    let user_config = manifest
        .get("userConfig")
        .filter(|value| !value.is_nullish());
    let channels = ChannelManifest::deserialize(manifest.clone()).ok();
    issues.extend(channel_issues(
        channels.as_ref(),
        &mcp_servers.value.servers,
    ));
    let listed: Vec<CommandFile> = commands
        .value
        .iter()
        .map(|file| CommandFile {
            path: file.path.clone(),
        })
        .collect();
    let debug = command_directory_debug(&context.text(), &listed).await;
    let plugin = Plugin {
        name: context.plugin_name,
        manifest: context.manifest.clone(),
        skills: skills.value,
        commands: commands.value,
        agents: agents.value,
        hooks: hooks.value,
        mcp_servers: mcp_servers.value.servers,
        lsp_servers: lsp_servers.value,
        output_styles: output_styles.value,
        workflows: workflows.value,
        themes: themes.value,
        monitors: monitors.value,
        bin: bin.value.files,
        settings: settings.value,
        mcp_bundles: mcp_servers.value.bundles,
        user_config: user_config
            .cloned()
            .unwrap_or_else(|| JsValue::Object(JsObject::new())),
        has_bin: bin.value.present,
    };
    LoadedPlugin {
        plugin: Some(plugin),
        issues,
        debug: (!debug.is_empty()).then_some(debug),
    }
}

crate::registry::exports! {
    "loading/load-plugin" {
        "loadManifest" => async ok |files: Files, signal: Option<Signal>|
            load_manifest(&files, signal.as_ref()).await;
        "loadPlugin" => async ok |input: LoadPluginInput| load_plugin(input).await;
    }
}
