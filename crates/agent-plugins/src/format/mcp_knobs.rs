//! Port of `src/format/mcp-knobs.ts`, switched.
//!
//! The environment variables that tune MCP servers and what each decides (env-vars.md, "Variables";
//! mcp.md, "Time limits on MCP tool calls", "Tool search for MCP server authors", "Option 3: Add a
//! local stdio server"). Pure over a lookup of the environment: reading the real one, and starting
//! anything, are the host's. The typed registry of every host variable lives with the environment
//! module; these are the semantics of the MCP rows.

use indexmap::IndexMap;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use super::env_number::{plain_positive_whole_env, positive_whole_env, whole_env};
use crate::js::JsValue;
use crate::js::value::serialize_number;
use crate::ports::BoxFuture;
use crate::registry::Callback;

/// What the host's `EnvLookup` (`mcp-env.ts`: `(name: string) => string | undefined`) answers: a
/// function of the host's, so its answer may wait. A Rust host passes a closure, answered at once.
pub trait EnvLookup: Send + Sync {
    /// The variable's value, `None` when it is unset.
    fn lookup<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>>;
}

impl<F: Fn(&str) -> Option<String> + Send + Sync> EnvLookup for F {
    fn lookup<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(std::future::ready(self(name)))
    }
}

impl EnvLookup for Callback {
    fn lookup<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(self.call(vec![JsValue::from(name)]))
    }
}

/// A positive whole number of a numeric variable (spec §3.6.1 item 2, J3.17); anything else is `undefined`.
#[must_use]
pub fn positive_whole(text: Option<&str>) -> Option<f64> {
    positive_whole_env(text)
}

/// "Timeout in milliseconds for MCP server startup (default: 30000, or 30 seconds)".
const DEFAULT_STARTUP_MS: f64 = 30_000.0;
/// "Timeout in milliseconds for MCP tool execution (default: 100000000, about 28 hours)".
const DEFAULT_TOOL_MS: f64 = 100_000_000.0;
/// "Each request also times out after 60 seconds by default".
const REQUEST_FLOOR_MS: f64 = 60_000.0;
/// "Values below 1000 are floored to one second" (the variable) and "ignored" (the per-server field).
const MIN_TOOL_MS: f64 = 1_000.0;
/// "Overrides the per-transport defaults of 300000 (5 minutes) for network servers and 1800000 (30 minutes) for stdio servers."
const NETWORK_IDLE_MS: f64 = 300_000.0;
const STDIO_IDLE_MS: f64 = 1_800_000.0;
/// "How long blocking MCP startup waits, in milliseconds, for the connection batch ... (default: 5000)".
const DEFAULT_CONNECT_MS: f64 = 5_000.0;

/// A number as the wire spells it, for an optional member that may hold `Infinity`.
fn serialize_some_number<S: Serializer>(
    number: &Option<f64>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match number {
        Some(number) => serialize_number(number, serializer),
        None => serializer.serialize_none(),
    }
}

/// The timeouts of one server.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTimeouts {
    /// `MCP_TIMEOUT`: one server's connect attempt.
    #[serde(serialize_with = "serialize_number")]
    pub startup_ms: f64,
    /// The wall-clock limit of one tool call.
    #[serde(serialize_with = "serialize_number")]
    pub tool_ms: f64,
    /// The per-request timer of an HTTP, SSE or connector server; none for stdio and WebSocket.
    #[serde(
        serialize_with = "serialize_some_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub request_ms: Option<f64>,
    /// How long a call may go with no response and no progress notification before it aborts; `undefined`
    /// when there is no idle check (`CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT` is `0`, or an IDE or SDK server).
    #[serde(
        serialize_with = "serialize_some_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub idle_ms: Option<f64>,
}

/// `McpTimeoutServer['transport']`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpServerKind {
    /// An HTTP server.
    Http,
    /// An SSE server.
    Sse,
    /// A claude.ai connector.
    Connector,
    /// A stdio server.
    Stdio,
    /// A WebSocket server.
    Ws,
    /// An IDE's server.
    Ide,
    /// An in-process SDK server.
    Sdk,
}

/// A server, as the timeouts see it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct McpTimeoutServer {
    /// `connector` is a claude.ai connector; `ide` and `sdk` are an IDE's server and an in-process SDK server.
    pub transport: McpServerKind,
    /// The per-server `timeout` as written; anything but a number of at least 1000 is ignored.
    #[serde(default)]
    pub timeout: JsValue,
}

/// Which servers have the second, per-request timer: "Stdio and WebSocket servers have no per-request timer."
const fn has_request_timer(transport: McpServerKind) -> bool {
    match transport {
        McpServerKind::Http | McpServerKind::Sse | McpServerKind::Connector => true,
        McpServerKind::Stdio | McpServerKind::Ws | McpServerKind::Ide | McpServerKind::Sdk => false,
    }
}

/// The idle window of each kind of server; "doesn't apply to IDE servers or SDK in-process servers".
const fn default_idle_ms(transport: McpServerKind) -> Option<f64> {
    match transport {
        McpServerKind::Http | McpServerKind::Sse | McpServerKind::Ws | McpServerKind::Connector => {
            Some(NETWORK_IDLE_MS)
        }
        McpServerKind::Stdio => Some(STDIO_IDLE_MS),
        McpServerKind::Ide | McpServerKind::Sdk => None,
    }
}

/// The limits `idle_window` takes: the tool timeout and the server's own `timeout`.
struct IdleLimits {
    tool_ms: f64,
    own: Option<f64>,
}

/// The idle window of one server. `CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT` replaces the default of its kind:
/// `0` turns the check off, "values below 1000 are raised to one second, and the value is capped at the
/// effective `MCP_TOOL_TIMEOUT`"; "a per-server `timeout` of at least 1000 raises that server's idle
/// window to at least the `timeout` value". A value that is not plain digits is ignored (judgment,
/// spec/judgments.md).
async fn idle_window(
    lookup: &dyn EnvLookup,
    transport: McpServerKind,
    limits: &IdleLimits,
) -> Option<f64> {
    let fallback = default_idle_ms(transport)?;
    let text = lookup.lookup("CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT").await;
    let written = whole_env(text.as_deref());
    if written == Some(0.0) {
        return None;
    }
    let set = written.map_or(fallback, |written| {
        written.max(MIN_TOOL_MS).min(limits.tool_ms)
    });
    Some(set.max(limits.own.unwrap_or(0.0)))
}

/// `MCP_TIMEOUT`, `MCP_TOOL_TIMEOUT` and the server's own `timeout`. The tool timeout is the per-server
/// `timeout` (of at least 1000), else `MCP_TOOL_TIMEOUT` (below 1000 floored to 1000), else the default.
/// The per-request timer is "the greatest of three values: 60 seconds, the tool timeout that applies to
/// the server, and `MCP_TIMEOUT`", where an unset `MCP_TOOL_TIMEOUT` does not count its default.
pub async fn mcp_timeouts(lookup: &dyn EnvLookup, server: &McpTimeoutServer) -> McpTimeouts {
    let startup_ms =
        positive_whole(lookup.lookup("MCP_TIMEOUT").await.as_deref()).unwrap_or(DEFAULT_STARTUP_MS);
    let text = lookup.lookup("MCP_TOOL_TIMEOUT").await;
    let read = whole_env(text.as_deref());
    let from_variable = read.map(|read| read.max(MIN_TOOL_MS));
    let own = server
        .timeout
        .as_f64()
        .filter(|timeout| *timeout >= MIN_TOOL_MS);
    let applies = own.or(from_variable);
    let tool_ms = applies.unwrap_or(DEFAULT_TOOL_MS);
    McpTimeouts {
        startup_ms,
        tool_ms,
        idle_ms: idle_window(lookup, server.transport, &IdleLimits { tool_ms, own }).await,
        request_ms: has_request_timer(server.transport)
            .then(|| REQUEST_FLOOR_MS.max(applies.unwrap_or(0.0)).max(startup_ms)),
    }
}

/// `MCP_CONNECT_TIMEOUT_MS`, or 5000.
pub async fn mcp_connect_timeout_ms(lookup: &dyn EnvLookup) -> f64 {
    positive_whole(lookup.lookup("MCP_CONNECT_TIMEOUT_MS").await.as_deref())
        .unwrap_or(DEFAULT_CONNECT_MS)
}

/// The limits on what a server tells Claude about itself and its tools.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpDescriptionLimits {
    /// A tool description that loads upfront.
    pub upfront: f64,
    /// A tool description loaded through tool search.
    pub deferred: f64,
    /// A server's instructions.
    pub instructions: f64,
}

/// "Server instructions, and the description of a tool that loads upfront: 4,096 characters"; "through tool search: 16,384".
const DEFAULT_DESCRIPTION_LIMITS: McpDescriptionLimits = McpDescriptionLimits {
    upfront: 4_096.0,
    deferred: 16_384.0,
    instructions: 4_096.0,
};

/// `CLAUDE_CODE_MAX_MCP_DESCRIPTION_LENGTH`: "Claude Code uses that number for both limits"; anything but plain digits is ignored.
pub async fn mcp_description_limits(lookup: &dyn EnvLookup) -> McpDescriptionLimits {
    let text = lookup
        .lookup("CLAUDE_CODE_MAX_MCP_DESCRIPTION_LENGTH")
        .await;
    plain_positive_whole_env(text.as_deref()).map_or(DEFAULT_DESCRIPTION_LIMITS, |value| {
        McpDescriptionLimits {
            upfront: value,
            deferred: value,
            instructions: value,
        }
    })
}

const fn is_high_surrogate(code: u16) -> bool {
    code >= 0xd800 && code <= 0xdbff
}

/// `slice(0, end)`'s end as a count of code units: a negative one counts from the end, `NaN` is 0.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a whole number from 0 to the length"
)]
fn slice_end(length: usize, end: f64) -> usize {
    let length_f = length as f64;
    let end = if end.is_nan() { 0.0 } else { end.trunc() };
    let end = if end < 0.0 {
        (length_f + end).max(0.0)
    } else {
        end.min(length_f)
    };
    end as usize
}

/// "Claude sees only the text before the cut"; the cut keeps a surrogate pair whole (judgment, spec/judgments.md).
#[must_use]
pub fn cut_mcp_description(text: &str, limit: f64) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    if (units.len() as f64) <= limit {
        return text.to_owned();
    }
    // `charCodeAt(limit - 1)`: a position that holds no unit (including `NaN`, which reads as 0) is `NaN`.
    let at = limit - 1.0;
    let at = if at.is_nan() { 0.0 } else { at.trunc() };
    let code = (at >= 0.0)
        .then(|| slice_end(units.len(), at))
        .and_then(|index| units.get(index));
    let end = if code.copied().is_some_and(is_high_surrogate) {
        limit - 1.0
    } else {
        limit
    };
    let kept = units.get(..slice_end(units.len(), end)).unwrap_or(&units);
    String::from_utf16_lossy(kept)
}

/// "For variables that turn a behavior on or off, set `1`, `true`, `yes`, or `on` ... `0`, `false`, `no`, or `off`, in any casing."
const SWITCHES: [(&str, bool); 8] = [
    ("1", true),
    ("true", true),
    ("yes", true),
    ("on", true),
    ("0", false),
    ("false", false),
    ("no", false),
    ("off", false),
];

/// `true`, `false`, or `undefined` for an unset, empty or unrecognised value.
#[must_use]
pub fn switch_of(text: Option<&str>) -> Option<bool> {
    let key = text?.to_lowercase();
    SWITCHES
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, on)| *on)
}

/// Judgment (spec/judgments.md): the docs say "a safe baseline environment" and do not list it. These are
/// the variables a process needs to find programs, its home and its locale, and none holds a credential;
/// the host passes its own list when it knows better.
pub const MCP_ENV_BASELINE: [&str; 13] = [
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "LANG",
    "LC_ALL",
    "TERM",
    "TMPDIR",
    "TEMP",
    "TMP",
    "SYSTEMROOT",
    "USERPROFILE",
];

/// What a stdio server's environment is made of.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStdioEnvironmentInput {
    /// The environment Claude Code runs in.
    pub snapshot: IndexMap<String, String>,
    /// The server's configured `env`, after expansion.
    pub configured: IndexMap<String, String>,
    /// "The project root": set as `CLAUDE_PROJECT_DIR` in the server's environment only.
    #[serde(default)]
    pub project_dir: Option<String>,
    /// What the plugin exports to its server: `ResolvedMcpServer.environment`, the path variables
    /// `CLAUDE_PLUGIN_ROOT` and `CLAUDE_PLUGIN_DATA` among them. Kept under the allowlist too: they are
    /// the client's own per-plugin values, not the inherited shell environment it withholds (spec J3.22).
    #[serde(default)]
    pub exported: Option<IndexMap<String, String>>,
    /// The names the allowlist keeps; `MCP_ENV_BASELINE` when unset.
    #[serde(default)]
    pub baseline: Option<Vec<String>>,
}

/// The environment a stdio server is spawned with. "`CLAUDE_CODE_MCP_ALLOWLIST_ENV` set to `1` spawns stdio
/// MCP servers with only a safe baseline environment plus the server's configured `env`, instead of
/// inheriting your shell environment"; `CLAUDE_PROJECT_DIR` is "set in the server's environment, not in
/// Claude Code's own environment".
#[must_use]
pub fn mcp_stdio_environment(input: &McpStdioEnvironmentInput) -> IndexMap<String, String> {
    let allowlist = switch_of(
        input
            .snapshot
            .get("CLAUDE_CODE_MCP_ALLOWLIST_ENV")
            .map(String::as_str),
    ) == Some(true);
    let names: Vec<&str> = input.baseline.as_ref().map_or_else(
        || MCP_ENV_BASELINE.to_vec(),
        |baseline| baseline.iter().map(String::as_str).collect(),
    );
    let mut environment: IndexMap<String, String> = input
        .snapshot
        .iter()
        .filter(|(name, _)| !allowlist || names.contains(&name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    for (name, value) in input.exported.iter().flatten() {
        environment.insert(name.clone(), value.clone());
    }
    if let Some(project_dir) = &input.project_dir {
        environment.insert("CLAUDE_PROJECT_DIR".to_owned(), project_dir.clone());
    }
    for (name, value) in &input.configured {
        environment.insert(name.clone(), value.clone());
    }
    environment
}

/// The session, as `claude_ai_connectors_fetched` reads it.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAiSession {
    /// Whether the user is logged in.
    pub logged_in: bool,
}

/// `ENABLE_CLAUDEAI_MCP_SERVERS`: "Set to `false` to stop Claude Code from fetching claude.ai MCP servers.
/// Enabled by default for logged-in users." A value that is not an on/off spelling leaves the default.
pub async fn claude_ai_connectors_fetched(
    lookup: &dyn EnvLookup,
    session: &ClaudeAiSession,
) -> bool {
    session.logged_in
        && switch_of(
            lookup
                .lookup("ENABLE_CLAUDEAI_MCP_SERVERS")
                .await
                .as_deref(),
        ) != Some(false)
}

crate::registry::exports! {
    "format/mcp-knobs" {
        "positiveWhole" => ok |text: Option<String>| positive_whole(text.as_deref());
        "mcpTimeouts" => async ok |lookup: Callback, server: McpTimeoutServer| mcp_timeouts(&lookup, &server).await;
        "mcpConnectTimeoutMs" => async ok |lookup: Callback| mcp_connect_timeout_ms(&lookup).await;
        "mcpDescriptionLimits" => async ok |lookup: Callback| mcp_description_limits(&lookup).await;
        "cutMcpDescription" => ok |text: String, limit: f64| cut_mcp_description(&text, limit);
        "switchOf" => ok |text: Option<String>| switch_of(text.as_deref());
        "MCP_ENV_BASELINE" => ok || MCP_ENV_BASELINE;
        "mcpStdioEnvironment" => ok |input: McpStdioEnvironmentInput| mcp_stdio_environment(&input);
        "claudeAiConnectorsFetched" => async ok |lookup: Callback, session: ClaudeAiSession| claude_ai_connectors_fetched(&lookup, &session).await;
    }
}
