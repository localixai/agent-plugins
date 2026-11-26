//! The core's counterpart of `src/ports/mcp-caller.ts` at sha256:29ffdb8f2b69, which stays TypeScript:
//! the host's interface (docs/RUST.md, "What stays TypeScript").
//! Calling a tool of a connected MCP server: the hooks' `mcp_tool` handler.

use crate::js::{JsObject, JsValue};
use serde::{Deserialize, Serialize};

use super::{BoxFuture, PortFailure, Signal};

/// Whether a server can be called.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpAvailability {
    /// Connected.
    Connected,
    /// Not connected within the wait.
    Unavailable,
}

/// One tool call.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCall {
    /// The server's name.
    pub server: String,
    /// The tool's name.
    pub tool: String,
    /// The tool's input.
    pub input: JsObject,
    /// After how long the host gives up.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Its cancellation.
    pub signal: Signal,
}

/// What a tool answered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    /// Whether the tool reported an error.
    pub is_error: bool,
    /// Its text content, joined.
    pub text: String,
    /// Its content blocks as the server sent them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<JsObject>>,
    /// Its structured content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<JsValue>,
}

/// The host's MCP connections.
pub trait McpCaller: Send + Sync {
    /// Waits up to `wait_ms` for the server to be connected.
    fn ensure_connected<'a>(
        &'a self,
        server: &'a str,
        wait_ms: u64,
        signal: &'a Signal,
    ) -> BoxFuture<'a, McpAvailability>;
    /// Calls one tool.
    fn call_tool(&self, call: McpToolCall) -> BoxFuture<'_, Result<McpToolResult, PortFailure>>;
}
