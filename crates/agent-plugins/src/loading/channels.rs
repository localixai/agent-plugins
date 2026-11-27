//! Port of `src/loading/channels.ts`, switched.
//!
//! A `channels` entry is "bound to one of the plugin's MCP servers": `server` is the "Key of the MCP
//! server in this plugin's `mcpServers` that the channel binds to" (plugins/manifest-reference.md,
//! "Channels"). The keys are those loading found, from `.mcp.json`, a file or the manifest.
//! Judgment: an entry bound to no server is an error issue, and the plugin still loads without that
//! channel (spec/judgments.md).

use indexmap::IndexMap;
use serde::Deserialize;

use crate::format::issues::{PluginIssue, error};
use crate::js::JsValue;

/// A `channels` entry of the manifest.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Channel {
    /// The key of the MCP server the channel binds to.
    pub server: String,
}

/// The part of the manifest this module reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct ChannelManifest {
    /// The manifest's `channels`.
    #[serde(default)]
    pub channels: Option<Vec<Channel>>,
}

/// The message of an entry bound to no server.
#[must_use]
pub fn channel_binding_message(server: &str) -> String {
    format!("Channel server \"{server}\" is not a key of this plugin's mcpServers")
}

/// One error per `channels` entry whose `server` is no key of `servers`.
#[must_use]
pub fn channel_issues(
    manifest: Option<&ChannelManifest>,
    servers: &IndexMap<String, JsValue>,
) -> Vec<PluginIssue> {
    manifest
        .and_then(|manifest| manifest.channels.as_ref())
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, channel)| !servers.contains_key(&channel.server))
        .map(|(index, channel)| {
            error(
                &format!("channels[{index}].server"),
                &channel_binding_message(&channel.server),
            )
        })
        .collect()
}

crate::registry::exports! {
    "loading/channels" {
        "channelBindingMessage" => ok |server: String| channel_binding_message(&server);
        "channelIssues" => ok |manifest: Option<ChannelManifest>, servers: IndexMap<String, JsValue>| channel_issues(manifest.as_ref(), &servers);
    }
}
