//! Port of `src/loading/resolve/processes.ts`, switched.
//!
//! The long-running processes a plugin declares, resolved for this host: MCP servers, language
//! servers and monitors. The kit never spawns any of them: the host starts each and reads the issues
//! for the ones that must not start (a `${user_config.*}` in a shell field, an unparseable URL).
//! An entry that does not start is left out and its issue is reported; the others are unaffected.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::channel::{PluginChannel, PluginServers, resolve_plugin_server};
use super::lsp::{ResolvedLspServer, resolve_lsp_server};
use super::mcp::ResolvedMcpServer;
use super::monitor::{ResolvedMonitor, resolve_monitor};
use super::substitute::scope::SubstitutionScope;
use crate::format::issues::PluginIssue;
use crate::format::lsp_config::LspServer;
use crate::format::monitors::Monitor;
use crate::js::{JsValue, promise_all, promise_all_pair};
use crate::ports::BoxFuture;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// What the resolvers read of a plugin (`Pick<Plugin, 'mcpServers' | 'mcpBundles' | 'lspServers' |
/// 'monitors' | 'manifest'>`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginProcesses {
    /// The MCP servers and their bundles.
    #[serde(flatten)]
    pub servers: PluginServers,
    /// The language servers by the name the plugin wrote.
    #[serde(default)]
    pub lsp_servers: IndexMap<String, LspServer>,
    /// The monitors, in declaration order.
    #[serde(default)]
    pub monitors: Vec<Monitor>,
    /// The manifest, when there is one: its `channels`.
    #[serde(default)]
    pub manifest: Option<ProcessManifest>,
}

/// What the resolvers read of a manifest: its channels.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ProcessManifest {
    /// The `channels` entries.
    #[serde(default)]
    pub channels: Option<Vec<PluginChannel>>,
}

/// The processes a plugin declares, resolved.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedProcesses {
    /// By the name the plugin wrote; each carries the `plugin:<plugin>:<server>` key `/mcp` shows.
    pub mcp_servers: IndexMap<String, ResolvedMcpServer>,
    /// By the name the plugin wrote; each carries its `pluginId`, to key it by across plugins.
    pub lsp_servers: IndexMap<String, ResolvedLspServer>,
    /// In declaration order; `monitorsToStart` decides when each starts.
    pub monitors: Vec<ResolvedMonitor>,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// One kind of process: what starts, by name for servers, and why the others do not.
#[derive(Clone, Debug, Serialize)]
pub struct ResolvedKind<R> {
    /// What starts.
    pub resolved: R,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// Resolves every entry of a record with one resolver; entries that do not start are left out.
async fn resolve_each<'a, V, R: Send + 'a>(
    entries: &'a IndexMap<String, V>,
    resolve: impl Fn(&'a str, &'a V) -> BoxFuture<'a, (Option<R>, Vec<PluginIssue>)>,
) -> ResolvedKind<IndexMap<String, R>> {
    let done = promise_all(
        entries
            .iter()
            .map(|(name, value)| -> BoxFuture<'a, _> {
                let one = resolve(name, value);
                Box::pin(async move { (name.clone(), one.await) })
            })
            .collect(),
    )
    .await;
    let mut resolved = IndexMap::new();
    let mut issues = Vec::new();
    for (name, (one, found)) in done {
        if let Some(one) = one {
            resolved.insert(name, one);
        }
        issues.extend(found);
    }
    ResolvedKind { resolved, issues }
}

/// The plugin's MCP servers. A bundled server resolves with its bundle's own settings; a server a channel binds to resolves with the channel's options
/// (manifest-reference.md, "Channels": "Saved values substitute into `${user_config.KEY}`
/// references in the server's `env`").
pub async fn resolve_mcp_servers<L: Locations + Clone, E: Environment + Clone>(
    plugin: &PluginProcesses,
    scope: &SubstitutionScope<L, E>,
) -> ResolvedKind<IndexMap<String, ResolvedMcpServer>> {
    let channels = plugin
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.channels.as_ref());
    resolve_each(
        &plugin.servers.mcp_servers,
        |name: &str, server: &JsValue| {
            let channel = channels.and_then(|all| all.iter().find(|one| one.server == name));
            Box::pin(async move {
                let done =
                    resolve_plugin_server(&plugin.servers, (name, server), channel, scope).await;
                (done.resolved, done.issues)
            })
        },
    )
    .await
}

/// The plugin's language servers.
pub async fn resolve_lsp_servers<L: Locations + Clone, E: Environment>(
    plugin: &PluginProcesses,
    scope: &SubstitutionScope<L, E>,
) -> ResolvedKind<IndexMap<String, ResolvedLspServer>> {
    resolve_each(&plugin.lsp_servers, |name: &str, server: &LspServer| {
        Box::pin(async move {
            let done = resolve_lsp_server(name, server, scope).await;
            (Some(done.resolved), done.issues)
        })
    })
    .await
}

/// The plugin's monitors.
pub async fn resolve_monitors<L: Locations + Clone, E: Environment>(
    plugin: &PluginProcesses,
    scope: &SubstitutionScope<L, E>,
) -> ResolvedKind<Vec<ResolvedMonitor>> {
    let done = promise_all(
        plugin
            .monitors
            .iter()
            .map(|monitor| -> BoxFuture<'_, _> { Box::pin(resolve_monitor(monitor, scope)) })
            .collect(),
    )
    .await;
    let mut resolved = Vec::new();
    let mut issues = Vec::new();
    for one in done {
        resolved.extend(one.resolved);
        issues.extend(one.issues);
    }
    ResolvedKind { resolved, issues }
}

/// Every process the plugin declares.
pub async fn resolve_processes<L: Locations + Clone, E: Environment + Clone>(
    plugin: &PluginProcesses,
    scope: &SubstitutionScope<L, E>,
) -> ResolvedProcesses {
    let ((mcp, lsp), monitors) = promise_all_pair(
        Box::pin(promise_all_pair(
            Box::pin(resolve_mcp_servers(plugin, scope)),
            Box::pin(resolve_lsp_servers(plugin, scope)),
        )),
        Box::pin(resolve_monitors(plugin, scope)),
    )
    .await;
    let mut issues = mcp.issues;
    issues.extend(lsp.issues);
    issues.extend(monitors.issues);
    ResolvedProcesses {
        mcp_servers: mcp.resolved,
        lsp_servers: lsp.resolved,
        monitors: monitors.resolved,
        issues,
    }
}

crate::registry::exports! {
    "loading/resolve/processes" {
        "resolveMcpServers" => async ok |plugin: PluginProcesses, scope: SubstitutionScope|
            resolve_mcp_servers(&plugin, &scope).await;
        "resolveLspServers" => async ok |plugin: PluginProcesses, scope: SubstitutionScope|
            resolve_lsp_servers(&plugin, &scope).await;
        "resolveMonitors" => async ok |plugin: PluginProcesses, scope: SubstitutionScope|
            resolve_monitors(&plugin, &scope).await;
        "resolveProcesses" => async ok |plugin: PluginProcesses, scope: SubstitutionScope|
            resolve_processes(&plugin, &scope).await;
    }
}
