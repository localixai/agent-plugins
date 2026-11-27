//! Port of `src/format/mcp-reserved.ts`, switched.
//!
//! The server names Claude Code reserves (mcp.md; spec §8.5 item 5): a server of the user's own
//! configuration under one of them is skipped. A plugin's server key is not checked, because the
//! server registers under its scoped name (J8.12), so plugin readers never call this.
//!
//! An `McpConfigRead` is the answer of `src/format/mcp-config.ts`, whose module is not in the core
//! yet; it is a `JsValue` here, and what the function answers keeps its members and their order.

use super::host_errors::mcp_server_name_reserved;
use super::issues::warning;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, in_property_order, property_order};

/// What `src/format/mcp-config.ts` read: `servers`, `issues` and the members it carries besides.
pub type McpConfigRead = JsValue;

/// The server names reserved.
pub const RESERVED_MCP_SERVER_NAMES: [&str; 5] = [
    "workspace",
    "claude-in-chrome",
    "computer-use",
    "Claude Preview",
    "Claude Browser",
];

fn is_reserved(name: &str) -> bool {
    RESERVED_MCP_SERVER_NAMES.contains(&name)
}

/// The servers of a user configuration (`~/.claude.json`, a project's `.mcp.json`) without those
/// under a reserved name, each reported as `"<name>" is a reserved MCP server name and was not
/// loaded` (warning). The match is exact, as the names are written.
#[must_use]
pub fn without_reserved_servers(read: &McpConfigRead, origin: &str) -> McpConfigRead {
    let JsValue::Object(members) = read else {
        return read.clone();
    };
    let Some(JsValue::Object(declared)) = members.get("servers") else {
        return read.clone();
    };
    let reserved: Vec<&String> = property_order(declared)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| is_reserved(name))
        .collect();
    if reserved.is_empty() {
        return read.clone();
    }
    let servers: JsObject = property_order(declared)
        .into_iter()
        .filter(|(name, _)| !is_reserved(name))
        .map(|(name, server)| (name.clone(), server.clone()))
        .collect();
    let mut issues: Vec<JsValue> = match members.get("issues") {
        Some(JsValue::Array(read_issues)) => read_issues.clone(),
        Some(_) | None => Vec::new(),
    };
    // A `PluginIssue` is plain data, so reading it into a value cannot fail.
    issues.extend(
        reserved
            .into_iter()
            .filter_map(|name| to_js_value(&warning(origin, &mcp_server_name_reserved(name))).ok()),
    );
    let mut answer = members.clone();
    answer.insert(
        "servers".to_owned(),
        JsValue::Object(in_property_order(servers)),
    );
    answer.insert("issues".to_owned(), JsValue::Array(issues));
    JsValue::Object(answer)
}

crate::registry::exports! {
    "format/mcp-reserved" {
        "RESERVED_MCP_SERVER_NAMES" => ok || RESERVED_MCP_SERVER_NAMES;
    }
}
