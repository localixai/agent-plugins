//! Port of `src/format/mcp-reload.ts`, switched.
//!
//! How a plugin's MCP servers follow a reload (mcp.md, "Plugin-provided MCP servers", "Automatic
//! lifecycle"; plugins/components.md, "Server names, tool names, and reloads"): "when the user runs
//! `/reload-plugins` ... a server whose configuration is unchanged keeps its connection. A server whose
//! configuration changed reconnects, and one you removed disconnects"; the same holds when the session
//! moves with `/cd` to a directory whose settings enable other plugins. Pure over the resolved configs;
//! connecting is the host's.

use serde::{Deserialize, Serialize};

use super::canonical_json::canonical_json;
use super::mcp_config::is_remote_mcp_server;
use crate::js::{JsObject, JsValue};

/// What a reload does, by server name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct McpReloadPlan {
    /// In both lists with the same configuration: the live connection stays.
    pub keep: Vec<String>,
    /// In both with a changed configuration.
    pub reconnect: Vec<String>,
    /// Only in the new list: a plugin was enabled, or a server added.
    pub connect: Vec<String>,
    /// Only in the old list: a plugin was disabled, or a server removed.
    pub disconnect: Vec<String>,
}

/// Whether two server configurations are the same data (key order and absent fields do not count).
fn same_mcp_configuration(left: &JsValue, right: &JsValue) -> bool {
    canonical_json(left) == canonical_json(right)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Keep,
    Reconnect,
    Connect,
}

/// What a reload does to each server, by the name the host registers it under. Names appear in the
/// lists in the order of the new map, then of the old one for `disconnect`.
#[must_use]
pub fn plan_mcp_reload(previous: &JsObject, next: &JsObject) -> McpReloadPlan {
    let status = |name: &str, server: &JsValue| match previous.get(name) {
        None | Some(JsValue::Undefined) => Status::Connect,
        Some(old) if same_mcp_configuration(old, server) => Status::Keep,
        Some(_) => Status::Reconnect,
    };
    let named = |wanted: Status| -> Vec<String> {
        next.iter()
            .filter(|(name, server)| status(name, server) == wanted)
            .map(|(name, _)| name.clone())
            .collect()
    };
    McpReloadPlan {
        keep: named(Status::Keep),
        reconnect: named(Status::Reconnect),
        connect: named(Status::Connect),
        disconnect: previous
            .keys()
            .filter(|name| !next.contains_key(*name))
            .cloned()
            .collect(),
    }
}

/// "A connection to that server failed less than 15 minutes earlier."
pub const MCP_FAILURE_SKIP_MS: f64 = 15.0 * 60_000.0;

/// errors.md quotes it as the status of a server that was not started.
pub const SKIPPING_CONNECTION: &str = "Skipping connection (recent failure cached; retries automatically in 15 min, or edit the plugin config to retry now)";

/// A failed connection.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConnectionFailure {
    /// When.
    pub at_ms: f64,
    /// The configuration the failed connection used.
    pub server: JsValue,
}

/// Whether Claude Code skips connecting a plugin's server: it is a stdio server whose connection failed
/// "less than 15 minutes earlier", with the configuration it has now ("or edit the plugin config to
/// retry now"). A remote server is not cached this way. Judgment (spec/judgments.md): a failure exactly
/// 15 minutes old is old enough, and one dated in the future does not count.
#[must_use]
pub fn skips_stdio_connection(
    server: &JsValue,
    failure: Option<&McpConnectionFailure>,
    now_ms: f64,
) -> bool {
    let Some(failure) = failure else {
        return false;
    };
    if is_remote_mcp_server(server) {
        return false;
    }
    let age = now_ms - failure.at_ms;
    (0.0..MCP_FAILURE_SKIP_MS).contains(&age) && same_mcp_configuration(server, &failure.server)
}

crate::registry::exports! {
    "format/mcp-reload" {
        "planMcpReload" => ok |previous: JsObject, next: JsObject| plan_mcp_reload(&previous, &next);
        "MCP_FAILURE_SKIP_MS" => ok || MCP_FAILURE_SKIP_MS;
        "SKIPPING_CONNECTION" => ok || SKIPPING_CONNECTION;
        "skipsStdioConnection" => ok |server: JsValue, failure: Option<McpConnectionFailure>, now_ms: f64|
            skips_stdio_connection(&server, failure.as_ref(), now_ms);
    }
}
