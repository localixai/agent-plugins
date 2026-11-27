//! The core's counterpart of `src/hooks/select/matcher-targets.ts` at sha256:1d0c3dea51d3, which stays
//! TypeScript: its table holds a reader function per event, which no value crossing the boundary
//! can carry (docs/RUST.md, "What stays TypeScript"). Only what the ported modules read is here:
//! whether an event's hooks have the session's MCP servers.
//!
//! Which input field an event's matcher filters on (hooks.md, "Matcher patterns", the table "Each
//! event type matches on a different field"), or none: "If you add a `matcher` field to an event
//! without matcher support, it is silently ignored".

use crate::js::JsValue;

/// `SessionStart` "at launch, including with `--continue` or `--resume`" fires before the MCP servers
/// are available; "after `/clear` or a compaction" they are. A fork (`--fork-session`) is a launch.
fn mcp_at_source(source: &str) -> Option<bool> {
    match source {
        "startup" | "resume" | "fork" => Some(false),
        "clear" | "compact" => Some(true),
        _ => None,
    }
}

/// Whether the session's MCP servers are available to the event's hooks (`target.mcpAvailable`),
/// `None` for an event whose target does not say (they are). "every `Setup` event" fires before
/// they are available.
#[must_use]
pub fn mcp_available(event: &str, facts: &JsValue) -> Option<bool> {
    match event {
        "SessionStart" => facts
            .get("source")
            .and_then(JsValue::as_str)
            .and_then(mcp_at_source),
        "Setup" => Some(false),
        _ => None,
    }
}
