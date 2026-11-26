//! Port of `src/format/core/mcp.ts`, switched.
//!
//! A core package's `mcp.json` (core §7.2): the outer object disables MCP for the package when it
//! fails (core §7.2.2 item 2); a server entry that fails is skipped and reported, the others load
//! (item 3); a valid entry whose transport the host does not support is skipped with a warning, since
//! missing support is not an error (item 4, core §11.3 item 4).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::document::{DocumentMembers, document_members};
use super::mcp_server::{
    CORE_TRANSPORTS, CoreMcpServer, CoreServerRead, CoreTransport, RemoteType, read_core_server,
};
use super::versions::{CoreDocument, CoreVersion};
use crate::format::issues::{PluginIssue, error, warning};
use crate::js::{JsValue, property_order};

/// What a host says about itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreMcpOptions {
    /// The extension namespaces the host implements (core §8.1).
    #[serde(default)]
    pub implemented: Option<Vec<String>>,
    /// The transports the host supports; all three by default (core §7.2.1, "Transport support").
    #[serde(default)]
    pub transports: Option<Vec<CoreTransport>>,
}

/// What reading an `mcp.json` gave.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreMcpResult {
    /// The servers kept, by name.
    pub servers: IndexMap<String, CoreMcpServer>,
    /// MCP is disabled for the package: the outer object failed.
    pub disabled: bool,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

const FILE: &str = "mcp.json";

/// The transport a server reads as: its `type`.
const fn transport_of(server: &CoreMcpServer) -> CoreTransport {
    match server {
        CoreMcpServer::Stdio(_) => CoreTransport::Stdio,
        CoreMcpServer::Remote(remote) => match remote.transport {
            RemoteType::StreamableHttp => CoreTransport::StreamableHttp,
            RemoteType::Sse => CoreTransport::Sse,
        },
    }
}

/// Reads a parsed `mcp.json` against the version `plugin.json` targets.
#[must_use]
pub fn read_core_mcp(
    raw: &JsValue,
    manifest_version: CoreVersion,
    options: &CoreMcpOptions,
) -> CoreMcpResult {
    let members = match document_members(raw, CoreDocument::Mcp, "mcpServers", manifest_version) {
        DocumentMembers::Problem(problem) => {
            return CoreMcpResult {
                servers: IndexMap::new(),
                disabled: true,
                issues: vec![error(FILE, &format!("MCP is disabled: {problem}"))],
            };
        }
        DocumentMembers::Members(members) => members,
    };
    let transports = options
        .transports
        .as_deref()
        .unwrap_or(CORE_TRANSPORTS.as_slice());
    let implemented = options.implemented.as_deref().unwrap_or_default();
    let mut servers = IndexMap::new();
    let mut issues = Vec::new();
    for (name, entry) in property_order(&members) {
        let path = format!("mcpServers.{name}");
        match read_core_server(entry, implemented) {
            CoreServerRead::Problem(problem) => issues.push(error(
                &path,
                &format!("Server \"{name}\" is skipped: {problem}"),
            )),
            CoreServerRead::Server(server) => {
                let transport = transport_of(&server);
                if transports.contains(&transport) {
                    servers.insert(name.clone(), server);
                } else {
                    issues.push(warning(
                        &path,
                        &format!(
                            "Server \"{name}\" is skipped: this client does not support \"{}\"",
                            transport.as_str()
                        ),
                    ));
                }
            }
        }
    }
    CoreMcpResult {
        servers,
        disabled: false,
        issues,
    }
}

crate::registry::exports! {
    "format/core/mcp" {
        "readCoreMcp" => ok |raw: JsValue, manifest_version: CoreVersion, options: Option<CoreMcpOptions>| read_core_mcp(&raw, manifest_version, &options.unwrap_or_default());
    }
}
