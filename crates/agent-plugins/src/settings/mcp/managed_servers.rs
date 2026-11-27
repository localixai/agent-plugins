//! Port of `src/settings/mcp/managed-servers.ts`, switched.
//!
//! The servers the organization provides through `managedMcpServers`, and what `/mcp`, `claude mcp
//! get` and `claude mcp remove` show of one.

use serde::Serialize;

use super::managed::{ManagedServerOptions, managed_server_entries};
use super::managed_entry::read_managed_entry;
use crate::format::issues::{PluginIssue, warning};
use crate::format::mcp_env::{McpDisplayScope, display_headers, display_reference};
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;
use crate::settings::layers::{PolicyLayer, PolicyScope};

/// The servers the organization provides and why an entry was not loaded.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProvidedServers {
    /// The servers the organization provides, as managed-scope candidates, in the order the sources define them.
    pub servers: Vec<JsValue>,
    /// Why an entry was not loaded: the notices `/status` shows.
    pub issues: Vec<PluginIssue>,
}

const KEY: &str = "managedMcpServers";

fn not_loaded(name: &str, reason: &str) -> PluginIssue {
    warning(
        &format!("{KEY}.{name}"),
        &format!("Managed MCP server \"{name}\" was not loaded: {reason}"),
    )
}

/// A managed layer whose value is no object: Claude Desktop's setting of the same name is an array of
/// another entry shape, and Claude Code "doesn't accept the array form and records a notice instead of
/// loading it" (managed-mcp.md, "What an entry can contain").
fn wrong_shape(layers: &[PolicyLayer], options: ManagedServerOptions) -> Vec<PluginIssue> {
    if options.desktop_managed.is_some() {
        return Vec::new();
    }
    layers
        .iter()
        .filter(|layer| layer.scope == PolicyScope::Managed)
        .filter_map(|layer| layer.settings.get(KEY))
        .filter(|value| !value.is_undefined() && !is_record(value))
        .map(|_| {
            warning(
                &format!("managed:{KEY}"),
                &format!(
                    "\"{KEY}\" must be an object keyed by server name; the array form of Claude Desktop is not accepted and no server was loaded from it"
                ),
            )
        })
        .collect()
}

/// The servers the organization provides through `managedMcpServers` (managed-mcp.md, "Provide servers
/// through managed settings"; v2.1.259): the managed layers' entries (`managedServerEntries`: managed
/// sources only, names across sources union, the higher source's entry whole), each kept only when it
/// passes every check of "What an entry can contain"; one that fails is dropped with a notice and the
/// others still load. Hand the result to `mcpLoadPlan`; it ranks above every other scope.
#[must_use]
pub fn read_managed_mcp_servers(
    layers: &[PolicyLayer],
    options: &ManagedServerOptions,
) -> ProvidedServers {
    let found = managed_server_entries(layers, options);
    let entries: JsObject = found.entries.into_iter().collect();
    let reads: Vec<(String, JsValue, JsValue)> = property_order(&entries)
        .into_iter()
        .map(|(name, raw)| {
            let read = read_managed_entry(name, raw);
            (name.clone(), read.server, read.reason)
        })
        .collect();
    let servers = reads
        .iter()
        .filter(|(_, server, _)| !server.is_undefined())
        .map(|(name, server, _)| {
            JsValue::Object(JsObject::from([
                ("scope".to_owned(), JsValue::String("managed".to_owned())),
                ("name".to_owned(), JsValue::String(name.clone())),
                ("server".to_owned(), server.clone()),
            ]))
        })
        .collect();
    let mut issues = found.issues;
    issues.extend(wrong_shape(layers, *options));
    issues.extend(
        reads
            .iter()
            .filter_map(|(name, _, reason)| reason.as_str().map(|reason| not_loaded(name, reason))),
    );
    ProvidedServers { servers, issues }
}

/// `/mcp` "lists provided servers under **Managed MCPs**".
pub const MANAGED_MCP_GROUP: &str = "Managed MCPs";

/// What `claude mcp get` and `/mcp` show of a provided server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedServerDescription {
    /// Always `Managed MCPs`.
    pub group: &'static str,
    /// "the URL's host only, for example `https://mcp.example.com/…`".
    pub url: String,
    /// "its header names without their values".
    pub header_names: Vec<String>,
}

/// What `claude mcp get` and `/mcp` show of a provided server (managed-mcp.md, "What users can see and change").
#[must_use]
pub fn describe_managed_server(server: &JsValue) -> ManagedServerDescription {
    ManagedServerDescription {
        group: MANAGED_MCP_GROUP,
        url: display_reference(server, McpDisplayScope::Managed),
        header_names: display_headers(server, McpDisplayScope::Managed)
            .into_iter()
            .map(|header| header.name)
            .collect(),
    }
}

/// What `claude mcp remove` reports for a provided server (managed-mcp.md, "How restrictions appear to users").
#[must_use]
pub fn managed_server_removal(name: &str) -> String {
    format!(
        "MCP server \"{name}\" is provided by your organization (managed settings) and cannot be removed locally."
    )
}

crate::registry::exports! {
    "settings/mcp/managed-servers" {
        "readManagedMcpServers" => ok |layers: Vec<PolicyLayer>, options: Option<ManagedServerOptions>|
            read_managed_mcp_servers(&layers, &options.unwrap_or_default());
        "MANAGED_MCP_GROUP" => ok || MANAGED_MCP_GROUP;
        "describeManagedServer" => ok |server: JsValue| describe_managed_server(&server);
        "managedServerRemoval" => ok |name: String| managed_server_removal(&name);
    }
}
