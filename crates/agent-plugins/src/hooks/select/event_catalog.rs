//! Port of `src/hooks/select/event-catalog.ts`, switched, switched.
//!
//! What the hooks reference says about each event beyond its input (hooks.md, "Hook lifecycle" and
//! "Prompt-based hooks"): its cadence, the handler types it supports, and the tool calls it skips.
//! Which field its matcher filters on is `matcher-targets.ts`.

use indexmap::IndexMap;
use serde::Serialize;

use crate::js::JsValue;
use crate::protocol::hook_config::HOOK_HANDLER_TYPES;
use crate::protocol::hook_events::HOOK_EVENTS;

/// "Events fall into three cadences": per session, per turn, and on every tool call. The other
/// events fire at their own moments (a compaction, a file change) and have no cadence of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cadence {
    /// Once per session.
    Session,
    /// Once per turn.
    Turn,
    /// On every tool call.
    ToolCall,
    /// At its own moment.
    Other,
}

/// What the reference says about one event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInfo {
    /// How often it fires.
    pub cadence: Cadence,
    /// The handler types it supports.
    pub handler_types: &'static [&'static str],
    /// Tools whose calls never fire the event: "PreToolUse and PostToolUse, except
    /// `EndConversation` calls, which skip both".
    pub skipped_tools: &'static [&'static str],
}

/// The events that support all five handler types.
const ALL: &[&str] = HOOK_HANDLER_TYPES;
/// `PermissionRequest`: "supports command, http, mcp_tool, and prompt hooks but not agent hooks".
const NO_AGENT: &[&str] = &["command", "http", "mcp_tool", "prompt"];
/// The events that "support command, http, and mcp_tool hooks but not prompt or agent".
const NO_MODEL: &[&str] = &["command", "http", "mcp_tool"];
/// `SessionStart` and `Setup`: command and mcp_tool only.
const LAUNCH: &[&str] = &["command", "mcp_tool"];

const NONE: &[&str] = &[];
const END_CONVERSATION: &[&str] = &["EndConversation"];

const fn info(
    cadence: Cadence,
    handler_types: &'static [&'static str],
    skipped_tools: &'static [&'static str],
) -> EventInfo {
    EventInfo {
        cadence,
        handler_types,
        skipped_tools,
    }
}

/// The catalog, in the TypeScript table's order.
pub const EVENT_CATALOG: [(&str, EventInfo); 33] = [
    ("SessionStart", info(Cadence::Session, LAUNCH, NONE)),
    ("SessionEnd", info(Cadence::Session, NO_MODEL, NONE)),
    ("UserPromptSubmit", info(Cadence::Turn, ALL, NONE)),
    ("Stop", info(Cadence::Turn, ALL, NONE)),
    ("StopFailure", info(Cadence::Turn, NO_MODEL, NONE)),
    ("PreToolUse", info(Cadence::ToolCall, ALL, END_CONVERSATION)),
    (
        "PostToolUse",
        info(Cadence::ToolCall, ALL, END_CONVERSATION),
    ),
    ("Setup", info(Cadence::Other, LAUNCH, NONE)),
    ("UserPromptExpansion", info(Cadence::Other, ALL, NONE)),
    ("PermissionRequest", info(Cadence::Other, NO_AGENT, NONE)),
    ("PermissionDenied", info(Cadence::Other, ALL, NONE)),
    ("PostToolUseFailure", info(Cadence::Other, ALL, NONE)),
    ("PostToolBatch", info(Cadence::Other, ALL, NONE)),
    ("Notification", info(Cadence::Other, NO_MODEL, NONE)),
    ("MessageDisplay", info(Cadence::Other, NO_MODEL, NONE)),
    ("SubagentStart", info(Cadence::Other, NO_MODEL, NONE)),
    ("SubagentStop", info(Cadence::Other, ALL, NONE)),
    ("TaskCreated", info(Cadence::Other, ALL, NONE)),
    ("TaskCompleted", info(Cadence::Other, ALL, NONE)),
    ("TeammateIdle", info(Cadence::Other, ALL, NONE)),
    ("InstructionsLoaded", info(Cadence::Other, NO_MODEL, NONE)),
    ("ConfigChange", info(Cadence::Other, NO_MODEL, NONE)),
    ("CwdChanged", info(Cadence::Other, NO_MODEL, NONE)),
    ("DirectoryAdded", info(Cadence::Other, NO_MODEL, NONE)),
    ("FileChanged", info(Cadence::Other, NO_MODEL, NONE)),
    ("WorktreeCreate", info(Cadence::Other, NO_MODEL, NONE)),
    ("WorktreeRemove", info(Cadence::Other, NO_MODEL, NONE)),
    ("PreCompact", info(Cadence::Other, NO_MODEL, NONE)),
    ("PostCompact", info(Cadence::Other, NO_MODEL, NONE)),
    ("PreModelSwitch", info(Cadence::Other, NO_MODEL, NONE)),
    ("PostModelSwitch", info(Cadence::Other, NO_MODEL, NONE)),
    ("Elicitation", info(Cadence::Other, NO_MODEL, NONE)),
    ("ElicitationResult", info(Cadence::Other, NO_MODEL, NONE)),
];

/// The catalog row of `event`.
fn info_of(event: &str) -> Option<&'static EventInfo> {
    EVENT_CATALOG
        .iter()
        .find(|(name, _)| *name == event)
        .map(|(_, row)| row)
}

/// Whether `event` runs handlers of this type; any other is skipped.
#[must_use]
pub fn supports_handler_type(event: &str, handler_type: &str) -> bool {
    info_of(event).is_some_and(|row| row.handler_types.contains(&handler_type))
}

/// The events that run handlers of this type.
#[must_use]
pub fn events_supporting(handler_type: &str) -> Vec<&'static str> {
    HOOK_EVENTS
        .iter()
        .copied()
        .filter(|event| supports_handler_type(event, handler_type))
        .collect()
}

/// Whether a call to `tool_name` fires `event` at all.
#[must_use]
pub fn fires_for_tool(event: &str, tool_name: &JsValue) -> bool {
    info_of(event).is_none_or(|row| {
        !tool_name
            .as_str()
            .is_some_and(|name| row.skipped_tools.contains(&name))
    })
}

/// The catalog as the TypeScript table, in its order.
fn event_catalog() -> IndexMap<&'static str, EventInfo> {
    EVENT_CATALOG.iter().copied().collect()
}

crate::registry::exports! {
    "hooks/select/event-catalog" {
        "EVENT_CATALOG" => ok event_catalog;
        "supportsHandlerType" => ok |event: String, handler_type: String| supports_handler_type(&event, &handler_type);
        "eventsSupporting" => ok |handler_type: String| events_supporting(&handler_type);
        "firesForTool" => ok |event: String, tool_name: JsValue| fires_for_tool(&event, &tool_name);
    }
}
