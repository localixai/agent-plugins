//! Port of `src/loading/discover.ts`, switched.
//!
//! Finds every component of a plugin folder through `PluginFiles` alone: the default locations and
//! the manifest's paths, nothing parsed.

use std::collections::HashSet;

use serde::Serialize;

use super::discover_config::{
    ConfigInput, ConfigKind, DiscoveredBin, Found, discover_bin, discover_config, discover_settings,
};
use super::discover_listed::{DiscoverContext, discover_listed};
use super::readers_text::DiscoveredFile;
use super::walk::ListedKind;
use crate::format::issues::PluginIssue;
use crate::format::load_files::Files;
use crate::format::locate::{LocateOptions, locate_context};
use crate::js::{JsValue, promise_all, promise_all_pair};
use crate::ports::BoxFuture;

/// What discovery found (`Discovered`): each kind's files or sources, and the issues.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovered {
    /// The skill files and folders.
    pub skills: Vec<DiscoveredFile>,
    /// The command files and inline commands.
    pub commands: Vec<DiscoveredFile>,
    /// The agent files.
    pub agents: Vec<DiscoveredFile>,
    /// The output style files.
    pub output_styles: Vec<DiscoveredFile>,
    /// The workflow files.
    pub workflows: Vec<DiscoveredFile>,
    /// The theme files.
    pub themes: Vec<DiscoveredFile>,
    /// The hooks sources.
    pub hooks: Vec<ConfigInput>,
    /// The LSP servers sources.
    pub lsp_servers: Vec<ConfigInput>,
    /// The monitors sources.
    pub monitors: Vec<ConfigInput>,
    /// The `settings.json` at the plugin root, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings_file: Option<String>,
    /// `bin/`.
    pub bin: DiscoveredBin,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
}

/// What decides whether a declared path leaves the plugin, the signal of the reads, and whether the
/// default `skills/` is scanned (`EntryMerge.scanDefaultSkills`; default true).
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverOptions {
    /// What `locate` needs.
    #[serde(flatten)]
    pub locate: LocateOptions,
    /// Whether the default `skills/` is scanned.
    #[serde(default)]
    pub scan_default_skills: Option<bool>,
}

const fn is_skill_folder(file: &DiscoveredFile) -> bool {
    file.directory.is_some()
}

/// The skill folders of the commands scan, then the `skills/` ones it did not already find: a
/// `commands` path naming `./skills/` loads each folder once (oracle 2.1.319, spec/judgments.md).
fn skill_folders(
    from_commands: &[DiscoveredFile],
    from_skills: Vec<DiscoveredFile>,
) -> Vec<DiscoveredFile> {
    let first: Vec<DiscoveredFile> = from_commands
        .iter()
        .filter(|file| is_skill_folder(file))
        .cloned()
        .collect();
    let seen: HashSet<Option<String>> = first.iter().map(|file| file.directory.clone()).collect();
    let rest = from_skills
        .into_iter()
        .filter(|file| !seen.contains(&file.directory));
    first.into_iter().chain(rest).collect()
}

/// Finds every component of a plugin folder (plugins/manifest-reference.md, "Standard layout", "Path
/// rules", "How each key combines with its default location") through `PluginFiles` alone: the
/// default locations and the manifest's paths, each key combined with its default by its own rule,
/// containment and existence checked, nothing parsed. `pluginName` is what components are namespaced
/// under and what names a skill at the plugin root; `locateContext` sets the platform default.
pub async fn discover_components(
    files: Files,
    manifest: JsValue,
    plugin_name: &str,
    options: DiscoverOptions,
) -> Discovered {
    let located = locate_context(files, options.locate);
    let context = DiscoverContext {
        files: located.files,
        platform: located.platform,
        roots: located.roots,
        signal: located.signal,
        manifest,
        plugin_name: plugin_name.to_owned(),
        scan_default_skills: options.scan_default_skills.unwrap_or(true),
    };
    let listed = |kind| -> BoxFuture<'_, _> { Box::pin(discover_listed(kind, &context)) };
    let config = context.config();
    let ((listed, (hooks, lsp_servers)), (monitors, (settings, bin))) = promise_all_pair(
        Box::pin(promise_all_pair(
            promise_all(vec![
                listed(ListedKind::Skills),
                listed(ListedKind::Commands),
                listed(ListedKind::Agents),
                listed(ListedKind::OutputStyles),
                listed(ListedKind::Workflows),
                listed(ListedKind::Themes),
            ]),
            Box::pin(promise_all_pair(
                Box::pin(discover_config(ConfigKind::Hooks, &config)),
                Box::pin(discover_config(ConfigKind::LspServers, &config)),
            )),
        )),
        Box::pin(promise_all_pair(
            Box::pin(discover_config(ConfigKind::Monitors, &config)),
            Box::pin(promise_all_pair(
                Box::pin(discover_settings(&config)),
                Box::pin(discover_bin(&config)),
            )),
        )),
    )
    .await;
    // `promise_all` answers one step per future given, in order.
    let mut each = listed.into_iter();
    let mut next = || {
        each.next().unwrap_or(Found {
            found: Vec::new(),
            issues: Vec::new(),
        })
    };
    let (skills, commands, agents) = (next(), next(), next());
    let (output_styles, workflows, themes) = (next(), next(), next());
    let mut issues: Vec<PluginIssue> = Vec::new();
    for step in [
        &skills.issues,
        &commands.issues,
        &agents.issues,
        &output_styles.issues,
    ] {
        issues.extend(step.iter().cloned());
    }
    for step in [
        &workflows.issues,
        &themes.issues,
        &hooks.issues,
        &lsp_servers.issues,
    ] {
        issues.extend(step.iter().cloned());
    }
    issues.extend(monitors.issues);
    issues.extend(settings.issues);
    Discovered {
        // A skill folder a commands scan found is a skill, ahead of `skills/` (oracle 2.1.319, walk.ts).
        skills: skill_folders(&commands.found, skills.found),
        commands: commands
            .found
            .into_iter()
            .filter(|file| !is_skill_folder(file))
            .collect(),
        agents: agents.found,
        output_styles: output_styles.found,
        workflows: workflows.found,
        themes: themes.found,
        hooks: hooks.found,
        lsp_servers: lsp_servers.found,
        monitors: monitors.found,
        settings_file: settings.found.into_iter().next(),
        bin,
        issues,
    }
}

crate::registry::exports! {
    "loading/discover" {
        "discoverComponents" => async ok |files: Files, manifest: JsValue, plugin_name: String, options: Option<DiscoverOptions>|
            discover_components(files, manifest, &plugin_name, options.unwrap_or_default()).await;
    }
}
