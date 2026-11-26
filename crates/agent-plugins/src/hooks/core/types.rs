//! The core's counterpart of `src/hooks/core/types.ts` at sha256:b1540a7667a8, which stays TypeScript: types
//! only, so there is nothing to call across the boundary (docs/RUST.md, "What stays TypeScript").
//!
//! What the core side of one hook occurrence is made of (Agent Plugins Extended §18.1.2): the core
//! input a core handler receives, the facts of a call only the host knows, and what each core
//! handler's run came to. Core §7.3 is cited as `core §n`.

use serde::{Deserialize, Serialize};

use crate::format::core::hooks::CoreHookEvent;
use crate::js::JsObject;

/// Who started a `UserPromptSubmit` turn (J18.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnOrigin {
    /// A submission from outside the session's own work.
    #[serde(rename = "external")]
    External,
    /// A turn the host started itself.
    #[serde(rename = "client")]
    Client,
}

/// What the host knows of a call that Claude Code's input does not carry, for the core tool
/// profiles (Extended §18.2.7). Each is optional: without it the profile that needs it is
/// omitted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreCallFacts {
    /// A shell command's exit code as the host observed it (`BashOutput` has none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<f64>,
    /// The tool name an MCP server declared, as sent in `tools/call` (`latest_tag`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_tool_name: Option<String>,
    /// The `CallToolResult` the MCP server returned, before any truncation or replacement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_result: Option<JsObject>,
    /// The text Claude received from a `Read` call (line numbers included; a notebook's cells).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_text: Option<String>,
    /// `UserPromptSubmit`: who started the turn (J18.7). `external`: a submission from outside
    /// the session's own work (the user, an SDK or `-p` caller, the poll-event channel, a message
    /// another session or a channel sends), which fires the core event; `client`: a turn the host
    /// started itself (a scheduled task, a `/loop` iteration, a task notification, an automatic
    /// continuation), which does not. Required for every turn whose `source` is `system`, which
    /// holds both kinds, whenever a core package has a `UserPromptSubmit` handler: the dispatch
    /// throws without it. Absent otherwise, the `source` decides and a turn without one is
    /// external.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_origin: Option<TurnOrigin>,
}

/// A portable tool profile: its name, its input projection and, after the call, its result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreToolProfile {
    /// The profile's name.
    pub name: String,
    /// Its input projection.
    pub input: JsObject,
    /// Its result, after the call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<JsObject>,
}

/// The core input of one occurrence and the event it is, with the tool name for notices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreOccurrence {
    /// The event.
    pub event: CoreHookEvent,
    /// The core input.
    pub input: JsObject,
    /// The tool name, for notices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

/// What a decision is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoreDecision {
    /// Let the call go on.
    Allow,
    /// Refuse it.
    Deny,
    /// Keep the turn going.
    Continue,
}

/// A successful core result: added context, or a decision with its reason (core §7.3.4–§7.3.8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CoreResult {
    /// Added context.
    Context {
        /// The text.
        context: String,
    },
    /// A decision.
    Decision {
        /// Which.
        decision: CoreDecision,
        /// Why.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

/// One core handler's run: its result, or why it failed (core §7.3.2 item 5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreRun {
    /// `core:<plugin id>:<Event>:<index>`.
    pub handler_id: String,
    /// The plugin.
    pub plugin_id: String,
    /// The command and its arguments as they ran, for the notice and the debug line.
    pub label: String,
    /// Absent with a failure, and when the handler expressed no opinion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<CoreResult>,
    /// Why it failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    /// Whether it timed out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timed_out: Option<bool>,
    /// Its exit code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<f64>,
    /// How long it was given.
    pub timeout_ms: f64,
}
