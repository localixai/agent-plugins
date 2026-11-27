//! Port of `src/loading/resolve/channel.ts`, switched.
//!
//! A channel of a plugin as the dialog and the host need it (plugins/manifest-reference.md,
//! "Channels"): the MCP server it binds to, the name shown in the configuration dialog title, the
//! options to prompt for, and the server's config with the saved values substituted into `env`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::bundle_scope::{BundleConfig, BundleScope, bundle_scope};
use super::mcp::{McpResolution, ResolvedMcpServer, resolve_mcp_server};
use super::substitute::scope::SubstitutionScope;
use super::user_config::ResolvedUserConfig;
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsObject, JsValue};
use crate::loading::channels::channel_binding_message;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// A `channels` entry of the manifest (`PluginChannel`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginChannel {
    /// The key of the plugin's MCP server the channel binds to.
    pub server: String,
    /// "Name shown in the configuration dialog title. Defaults to the server name".
    #[serde(default)]
    pub display_name: Option<String>,
    /// The options to prompt for, in the shape of the top-level `userConfig`.
    #[serde(default)]
    pub user_config: Option<JsObject>,
}

/// What the resolvers read of a plugin (`Pick<Plugin, 'mcpServers' | 'mcpBundles'>`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginServers {
    /// The MCP servers by the name the plugin wrote.
    #[serde(default)]
    pub mcp_servers: IndexMap<String, JsValue>,
    /// The bundled servers' bundles, by server name.
    #[serde(default)]
    pub mcp_bundles: IndexMap<String, BundleConfig>,
}

/// A channel of a plugin as the dialog and the host need it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedChannel {
    /// The key of the plugin's MCP server the channel binds to.
    pub server: String,
    /// "Name shown in the configuration dialog title. Defaults to the server name".
    pub display_name: String,
    /// The options to prompt for, in the shape of the top-level `userConfig`.
    pub user_config: JsObject,
    /// The bound server resolved with the channel's options; undefined when it does not start.
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub mcp: Option<ResolvedMcpServer>,
}

/// One channel resolved.
#[derive(Clone, Debug, Serialize)]
pub struct ChannelResolution {
    /// Undefined when the channel is bound to no server of the plugin.
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub resolved: Option<ResolvedChannel>,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// The scope of the server a channel binds to: the channel's options join the plugin's, a channel
/// option winning over a top-level one of the same key (judgment, spec/judgments.md).
fn channel_scope<L: Clone, E: Clone>(
    scope: &SubstitutionScope<L, E>,
    channel: &PluginChannel,
) -> SubstitutionScope<L, E> {
    let mut declared = scope.user_config.declared.clone();
    for (key, value) in channel.user_config.iter().flatten() {
        declared.insert(key.clone(), value.clone());
    }
    SubstitutionScope {
        user_config: ResolvedUserConfig {
            declared,
            values: scope.user_config.values.clone(),
        },
        ..scope.clone()
    }
}

/// Resolves one MCP server of a plugin, the one rule both `resolveMcpServers` and `resolveChannel`
/// take: a bundled server with its bundle's own settings only (`bundleScope`, which may keep it from
/// starting; a channel's options do not join them: judgment, spec/judgments.md), any other with the
/// options of the channel that binds it joined to the plugin's.
pub async fn resolve_plugin_server<L: Locations + Clone, E: Environment + Clone>(
    plugin: &PluginServers,
    (name, server): (&str, &JsValue),
    channel: Option<&PluginChannel>,
    scope: &SubstitutionScope<L, E>,
) -> McpResolution {
    let Some(bundle) = plugin.mcp_bundles.get(name) else {
        return match channel {
            Some(channel) => resolve_mcp_server(name, server, &channel_scope(scope, channel)).await,
            None => resolve_mcp_server(name, server, scope).await,
        };
    };
    match bundle_scope(scope, name, bundle) {
        BundleScope::Issue { issue } => McpResolution {
            resolved: None,
            issues: vec![issue],
            not_configured: None,
        },
        BundleScope::Scope { scope } => resolve_mcp_server(name, server, &scope).await,
    }
}

/// Resolves one channel against the plugin's servers. "Saved values substitute into
/// `${user_config.KEY}` references in the server's `env`": the channel's options join the plugin's
/// for this server, so `${user_config.bot_token}` in `env` takes the channel's `bot_token` (a
/// channel option wins over a top-level one of the same key; judgment, spec/judgments.md). A bundled
/// server reads its bundle's settings (`Plugin.mcpBundles`), as it does in `resolveMcpServers`.
pub async fn resolve_channel<L: Locations + Clone, E: Environment + Clone>(
    channel: &PluginChannel,
    plugin: &PluginServers,
    scope: &SubstitutionScope<L, E>,
) -> ChannelResolution {
    let Some(server) = plugin.mcp_servers.get(&channel.server) else {
        let issue = error("channels.server", &channel_binding_message(&channel.server));
        return ChannelResolution {
            resolved: None,
            issues: vec![issue],
        };
    };
    let user_config = channel.user_config.clone().unwrap_or_default();
    let bound =
        resolve_plugin_server(plugin, (&channel.server, server), Some(channel), scope).await;
    ChannelResolution {
        resolved: Some(ResolvedChannel {
            server: channel.server.clone(),
            display_name: channel
                .display_name
                .clone()
                .unwrap_or_else(|| channel.server.clone()),
            user_config,
            mcp: bound.resolved,
        }),
        issues: bound.issues,
    }
}

crate::registry::exports! {
    "loading/resolve/channel" {
        "resolveChannel" => async ok |channel: PluginChannel, plugin: PluginServers, scope: SubstitutionScope|
            resolve_channel(&channel, &plugin, &scope).await;
    }
}
