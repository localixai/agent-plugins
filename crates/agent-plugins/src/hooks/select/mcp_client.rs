//! Port of `src/hooks/select/mcp-client.ts`, switched.
//!
//! hooks.md, "Events that fire before MCP servers are available": `SessionStart` at launch and every
//! `Setup` event "skip their `mcp_tool` hooks without calling the tool", and "the debug log records
//! `mcp_tool hooks are not available for the 'SessionStart' hook event (no MCP client context)`".

use super::matcher_targets::mcp_available;
use crate::js::JsValue;

/// The handler types that call an MCP tool, so need the session's MCP servers.
fn calls_mcp(kind: &str) -> bool {
    matches!(kind, "mcp_tool")
}

/// Whether a handler of `type` is skipped on this occurrence of `event` for want of an MCP client.
#[must_use]
pub fn lacks_mcp_client(event: &str, facts: &JsValue, kind: &str) -> bool {
    calls_mcp(kind) && mcp_available(event, facts) == Some(false)
}

/// What the skip is reported as: the docs' own debug-log line (judgment: a warning, spec/judgments.md).
#[must_use]
pub fn no_mcp_client_message(event: &str) -> String {
    format!(
        "mcp_tool hooks are not available for the '{event}' hook event (no MCP client context) and are skipped"
    )
}

crate::registry::exports! {
    "hooks/select/mcp-client" {
        "lacksMcpClient" => ok |event: String, facts: JsValue, kind: String|
            lacks_mcp_client(&event, &facts, &kind);
        "noMcpClientMessage" => ok |event: String| no_mcp_client_message(&event);
    }
}
