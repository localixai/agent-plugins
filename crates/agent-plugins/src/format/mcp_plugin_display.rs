//! Port of `src/format/mcp-plugin-display.ts`, switched.
//!
//! What `claude mcp get` and `claude mcp list` show of a plugin's servers (mcp.md, "Plugin-provided MCP
//! servers"): "`claude mcp get` hides the values configured for a plugin's stdio or WebSocket server,
//! because they can carry credentials". Pure data: the lines, not the terminal.

use serde::Serialize;

use super::mcp_config::mcp_transport_of;
use crate::js::{JsValue, string_of};

/// A plugin's server as `claude mcp get` and `claude mcp list` show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RedactedPluginServer {
    /// The lines `claude mcp get` prints for the command, the URL, the arguments and the variables.
    pub get: Vec<String>,
    /// What `claude mcp list` shows in place of the command or URL; absent where the docs name none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<String>,
}

const REDACTED: &str = "[REDACTED]";

/// The keys of a member that is an object (`Object.keys(x ?? {})`).
fn keys_of(server: &JsValue, key: &str) -> Vec<String> {
    server
        .get(key)
        .and_then(JsValue::as_object)
        .map(|members| members.keys().cloned().collect())
        .unwrap_or_default()
}

fn redacted_stdio(server: &JsValue) -> Option<RedactedPluginServer> {
    server.get("command")?;
    // "It prints `Command: stdio`, an empty `Args:` line, and each environment variable as `NAME=[REDACTED]`."
    let mut get = vec!["Command: stdio".to_owned(), "Args:".to_owned()];
    get.extend(
        keys_of(server, "env")
            .iter()
            .map(|name| format!("{name}={REDACTED}")),
    );
    Some(RedactedPluginServer { get, list: None })
}

fn redacted_ws(server: &JsValue) -> Option<RedactedPluginServer> {
    server.get("url")?;
    // "It prints `URL: ws` and each header as `<name>: [REDACTED]`, and `claude mcp list` shows `ws`."
    let mut get = vec!["URL: ws".to_owned()];
    get.extend(
        keys_of(server, "headers")
            .iter()
            .map(|name| format!("{name}: {REDACTED}")),
    );
    Some(RedactedPluginServer {
        get,
        list: Some("ws".to_owned()),
    })
}

/// `REDACTORS`, one per transport.
fn redactors(transport: &str, server: &JsValue) -> Option<RedactedPluginServer> {
    match transport {
        "stdio" => redacted_stdio(server),
        "ws" => redacted_ws(server),
        // The values of an http or sse server are not among those the docs say are hidden.
        _ => None,
    }
}

/// The redacted view of a plugin's stdio or WebSocket server, or `undefined` for an http or sse server,
/// whose values the docs do not say are hidden.
#[must_use]
pub fn redact_plugin_server(server: &JsValue) -> Option<RedactedPluginServer> {
    redactors(&string_of(&mcp_transport_of(server)), server)
}

crate::registry::exports! {
    "format/mcp-plugin-display" {
        "redactPluginServer" => ok |server: JsValue| redact_plugin_server(&server);
    }
}
