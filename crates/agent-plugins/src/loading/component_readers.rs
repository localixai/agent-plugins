//! The core's counterpart of `src/loading/component-readers.ts` at sha256:1b357850b4cf, which stays
//! TypeScript: its export, `COMPONENT_READERS`, is a public table of functions, which no value
//! crossing the boundary can be; each of its readers is a wrapper over the core. The core keeps
//! this table for its own loads (`load_plugin.rs`).
//!
//! The integration seam of loading: one reader per component kind, typed over the kinds, so a kind
//! added to the layout fails the build until it has one. Each reads what discovery found.
//!
//! `skills`, `commands`, `agents` and `outputStyles` keep the path, name and text of each file: the
//! streams that own those formats (skills, agents, content) replace the entry with a reader that
//! parses it, and nothing else in loading changes. `hooks`, `mcpServers`, `lspServers`, `monitors`
//! and `settings` read their JSON through the format readers; `workflows` and `themes` read the
//! `meta` block and the theme file; `bin` hands over the listing.

use indexmap::IndexMap;

use super::command_debug;
use super::discover::Discovered;
use super::discover_config::DiscoveredBin;
use super::readers_assets::{
    AssetContext, ComponentRead, DiscoveredAssets, PluginTheme, PluginWorkflow, read_themes,
    read_workflows,
};
use super::readers_config::{
    ConfigContext, DiscoveredConfig, McpServersValue, read_hooks, read_lsp_servers,
    read_mcp_servers, read_monitor_list, read_settings,
};
use super::readers_text::{DiscoveredFile, PluginTextFile, text_files};
use crate::format::hooks_settings::HooksSettings;
use crate::format::load_files::Files;
use crate::format::lsp_config::LspServer;
use crate::format::mcp_sources::CollectOptions;
use crate::format::monitors::Monitor;
use crate::js::{JsObject, JsValue};
use crate::ports::{BoxFuture, Signal};

/// What every reader is given (`ReaderContext` of `src/loading/reader-types.ts`, a module of types
/// only): the load's files, the manifest, the name, what discovery found and what MCP needs.
#[derive(Clone, Debug)]
pub struct ReaderContext {
    /// The plugin folder as the load reads it.
    pub files: Files,
    /// The manifest; `undefined` without one.
    pub manifest: JsValue,
    /// The name components are namespaced under.
    pub plugin_name: String,
    /// What discovery found.
    pub discovered: Discovered,
    /// What contains MCP paths, and the host's bundle opener.
    pub mcp: CollectOptions,
    /// Hook events a marketplace entry's inline hooks replace.
    pub replaced_hook_events: Vec<String>,
    /// Aborts the reads.
    pub signal: Option<Signal>,
}

impl ReaderContext {
    /// What the configuration readers read of it.
    fn config(&self) -> ConfigContext {
        ConfigContext {
            files: self.files.clone(),
            manifest: self.manifest.clone(),
            discovered: DiscoveredConfig {
                hooks: self.discovered.hooks.clone(),
                lsp_servers: self.discovered.lsp_servers.clone(),
                monitors: self.discovered.monitors.clone(),
                settings_file: self.discovered.settings_file.clone(),
            },
            mcp: self.mcp.clone(),
            replaced_hook_events: self.replaced_hook_events.clone(),
            signal: self.signal.clone(),
        }
    }

    /// What the theme and workflow readers read of it.
    fn assets(&self) -> AssetContext {
        AssetContext {
            files: self.files.clone(),
            plugin_name: self.plugin_name.clone(),
            discovered: DiscoveredAssets {
                themes: self.discovered.themes.clone(),
                workflows: self.discovered.workflows.clone(),
            },
            signal: self.signal.clone(),
        }
    }

    /// What the text readers and the command line read of it.
    #[must_use]
    pub fn text(&self) -> command_debug::ReaderContext<Files> {
        command_debug::ReaderContext {
            files: self.files.clone(),
            manifest: Some(self.manifest.clone()),
            plugin_name: self.plugin_name.clone(),
            signal: self.signal.clone(),
        }
    }
}

/// One component's reader (`ComponentReader<K>`).
pub type ComponentReader<T> = for<'a> fn(&'a ReaderContext) -> BoxFuture<'a, ComponentRead<T>>;

/// One reader per component kind (`{ readonly [K in ReadKind]: ComponentReader<K> }`).
pub struct ComponentReaders {
    /// `skills`.
    pub skills: ComponentReader<Vec<PluginTextFile>>,
    /// `commands`.
    pub commands: ComponentReader<Vec<PluginTextFile>>,
    /// `agents`.
    pub agents: ComponentReader<Vec<PluginTextFile>>,
    /// `outputStyles`.
    pub output_styles: ComponentReader<Vec<PluginTextFile>>,
    /// `workflows`.
    pub workflows: ComponentReader<Vec<PluginWorkflow>>,
    /// `themes`.
    pub themes: ComponentReader<Vec<PluginTheme>>,
    /// `hooks`.
    pub hooks: ComponentReader<HooksSettings>,
    /// `mcpServers`.
    pub mcp_servers: ComponentReader<McpServersValue>,
    /// `lspServers`.
    pub lsp_servers: ComponentReader<IndexMap<String, LspServer>>,
    /// `monitors`.
    pub monitors: ComponentReader<Vec<Monitor>>,
    /// `settings`.
    pub settings: ComponentReader<JsObject>,
    /// `bin`.
    pub bin: ComponentReader<DiscoveredBin>,
}

/// `textFiles(kind, find)`: the text of the files `find` picks of what discovery found.
fn text_of<'a>(
    kind: &'static str,
    context: &'a ReaderContext,
    found: &'a [DiscoveredFile],
) -> BoxFuture<'a, ComponentRead<Vec<PluginTextFile>>> {
    Box::pin(async move {
        let read = text_files(kind, found, &context.text()).await;
        ComponentRead {
            value: read.value,
            issues: read.issues,
            rejects_plugin: None,
        }
    })
}

fn skills(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<Vec<PluginTextFile>>> {
    text_of("skills", context, &context.discovered.skills)
}

fn commands(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<Vec<PluginTextFile>>> {
    text_of("commands", context, &context.discovered.commands)
}

fn agents(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<Vec<PluginTextFile>>> {
    text_of("agents", context, &context.discovered.agents)
}

fn output_styles(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<Vec<PluginTextFile>>> {
    text_of("outputStyles", context, &context.discovered.output_styles)
}

fn workflows(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<Vec<PluginWorkflow>>> {
    Box::pin(async move { read_workflows(&context.assets()).await })
}

fn themes(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<Vec<PluginTheme>>> {
    Box::pin(async move { read_themes(&context.assets()).await })
}

fn hooks(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<HooksSettings>> {
    Box::pin(async move { read_hooks(&context.config()).await })
}

fn mcp_servers(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<McpServersValue>> {
    Box::pin(async move { read_mcp_servers(&context.config()).await })
}

fn lsp_servers(
    context: &ReaderContext,
) -> BoxFuture<'_, ComponentRead<IndexMap<String, LspServer>>> {
    Box::pin(async move { read_lsp_servers(&context.config()).await })
}

fn monitors(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<Vec<Monitor>>> {
    Box::pin(async move { read_monitor_list(&context.config()).await })
}

fn settings(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<JsObject>> {
    Box::pin(async move { read_settings(&context.config()).await })
}

fn bin(context: &ReaderContext) -> BoxFuture<'_, ComponentRead<DiscoveredBin>> {
    let value = context.discovered.bin.clone();
    Box::pin(async move {
        ComponentRead {
            value,
            issues: Vec::new(),
            rejects_plugin: None,
        }
    })
}

/// The readers, one per kind.
pub const COMPONENT_READERS: ComponentReaders = ComponentReaders {
    skills,
    commands,
    agents,
    output_styles,
    workflows,
    themes,
    hooks,
    mcp_servers,
    lsp_servers,
    monitors,
    settings,
    bin,
};
