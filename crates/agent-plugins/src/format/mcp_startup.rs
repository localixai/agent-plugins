//! Port of `src/format/mcp-startup.ts`, switched.
//!
//! The environment variables that decide how MCP servers connect at startup and how long a call runs
//! before it moves to the background (env-vars.md, "Variables"; mcp.md, "Automatic backgrounding of long
//! tool calls", "Server status detail", "MCP client runtimes"; agent-sdk/mcp.md, "Connection timing").
//! Pure over a lookup of the environment: connecting, the cache on disk and the clock are the host's.
//! A value that is not a number (digits, `2e3`, `64_000`: spec §3.6.1 item 2, J3.17) is ignored.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::env_number::whole_env;
use super::mcp_elicitation::NEW_REVISION;
use super::mcp_env::EnvLookup;
use super::mcp_knobs::{McpServerKind, McpTimeoutServer, mcp_timeouts, positive_whole, switch_of};
use crate::js::{JsValue, compare_code_units, math_max, math_min};
use crate::registry::Callback;

/// "Maximum number of local MCP servers (stdio) to connect in parallel during startup (default: 3)".
const DEFAULT_LOCAL_BATCH: f64 = 3.0;
/// "Maximum number of remote MCP servers (HTTP/SSE) to connect in parallel during startup (default: 20)".
const DEFAULT_REMOTE_BATCH: f64 = 20.0;

/// Servers connecting at once.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct McpConnectionBatches {
    /// `MCP_SERVER_CONNECTION_BATCH_SIZE`: stdio servers connecting at once.
    pub local: f64,
    /// `MCP_REMOTE_SERVER_CONNECTION_BATCH_SIZE`: HTTP and SSE servers connecting at once.
    pub remote: f64,
}

async fn variable(lookup: &dyn EnvLookup, name: &str) -> Option<String> {
    lookup.lookup(name).await
}

/// The batch sizes of startup.
pub async fn mcp_connection_batches(lookup: &dyn EnvLookup) -> McpConnectionBatches {
    let local = variable(lookup, "MCP_SERVER_CONNECTION_BATCH_SIZE").await;
    let remote = variable(lookup, "MCP_REMOTE_SERVER_CONNECTION_BATCH_SIZE").await;
    McpConnectionBatches {
        local: positive_whole(local.as_deref()).unwrap_or(DEFAULT_LOCAL_BATCH),
        remote: positive_whole(remote.as_deref()).unwrap_or(DEFAULT_REMOTE_BATCH),
    }
}

/// "An MCP tool call in the main conversation that is still running after two minutes moves to a background task."
const DEFAULT_BACKGROUND_MS: f64 = 120_000.0;

/// When a call moves to the background.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpBackgrounding {
    /// Milliseconds before a running call moves to a background task; `undefined` when that is off.
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub after_ms: Option<f64>,
    /// In an interactive session Claude waits for the result of an MCP task instead of working on.
    pub waits_for_tasks: bool,
}

/// A whole number, `0` included, of a numeric variable (spec §3.6.1 item 2); anything else is `undefined`.
fn whole_or_zero(text: Option<&str>) -> Option<f64> {
    whole_env(text)
}

/// `CLAUDE_CODE_MCP_AUTO_BACKGROUND_MS`: "Set to `0` to turn automatic backgrounding off and, in an
/// interactive session, to make Claude wait for MCP tasks"; `CLAUDE_CODE_DISABLE_MCP_TASK_BACKGROUND` set
/// to `1` makes Claude wait for them while backgrounding stays on.
pub async fn mcp_backgrounding(lookup: &dyn EnvLookup) -> McpBackgrounding {
    let text = variable(lookup, "CLAUDE_CODE_MCP_AUTO_BACKGROUND_MS").await;
    let written = whole_or_zero(text.as_deref());
    let off = written == Some(0.0);
    let waits = if off {
        true
    } else {
        let text = variable(lookup, "CLAUDE_CODE_DISABLE_MCP_TASK_BACKGROUND").await;
        switch_of(text.as_deref()) == Some(true)
    };
    McpBackgrounding {
        after_ms: if off {
            None
        } else {
            Some(written.unwrap_or(DEFAULT_BACKGROUND_MS))
        },
        waits_for_tasks: waits,
    }
}

/// One server at startup.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStartupServer {
    /// `alwaysLoad: true`: "their tools must be present when the first prompt is built".
    pub always_load: bool,
    /// Its tool list comes from a valid discovery-cache entry: it connects on its first tool call.
    pub cached: bool,
    /// Its transport: only an `http` or `sse` server is served from the discovery cache (spec §8.8 item 1).
    pub transport: String,
}

/// Which transports a discovery-cache entry serves: "a remote (`http` or `sse`) server".
const CACHEABLE: &[(&str, bool)] = &[
    ("stdio", false),
    ("sse", true),
    ("http", true),
    ("ws", false),
];

/// The session's mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStartupRun {
    /// `claude -p` without `--input-format stream-json`.
    pub print_mode: bool,
}

/// Whether startup waits for one server before the first query. "MCP startup is non-blocking by default";
/// `MCP_CONNECTION_NONBLOCKING=0` waits for every server. A server with `alwaysLoad: true` "still make\[s\]
/// startup wait regardless, except when served from the discovery cache", and `claude -p` "also waits for
/// still-pending servers before the first turn regardless of this variable". Judgment (spec/judgments.md):
/// a cached server is not connected at startup, so it is never waited for.
pub async fn mcp_startup_waits_for(
    lookup: &dyn EnvLookup,
    server: &McpStartupServer,
    run: McpStartupRun,
) -> bool {
    let cacheable = CACHEABLE
        .iter()
        .any(|(transport, serves)| *serves && *transport == server.transport);
    if server.cached && cacheable {
        return false;
    }
    let text = variable(lookup, "MCP_CONNECTION_NONBLOCKING").await;
    let blocking = switch_of(text.as_deref()) == Some(false);
    blocking || server.always_load || run.print_mode
}

/// What keeps its own first-turn wait in a non-interactive session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpFirstTurnServer {
    /// The `permissionPromptToolName` server.
    PermissionPrompt,
    /// An in-process SDK server.
    Sdk,
    /// A named server.
    Named,
    /// Any other.
    Other,
}

/// "A still-connecting named server: at least up to 2 seconds, so a value below 2000 doesn't shorten its wait."
const NAMED_SERVER_FLOOR_MS: f64 = 2_000.0;

/// `CLAUDE_CODE_MCP_STARTUP_WAIT_MS`: how long the first turn waits for a server that is still
/// connecting, in place of the default wait (`undefined` when the variable is unset: the host's default
/// applies). `0` skips the wait. "Three waits apply whatever value you set, including `0`": the
/// `permissionPromptToolName` server and an in-process SDK server wait up to `MCP_TIMEOUT`, a named one
/// at least 2 seconds.
pub async fn mcp_first_turn_wait_ms(
    lookup: &dyn EnvLookup,
    server: McpFirstTurnServer,
) -> Option<f64> {
    let text = variable(lookup, "CLAUDE_CODE_MCP_STARTUP_WAIT_MS").await;
    let written = whole_or_zero(text.as_deref())?;
    Some(match server {
        McpFirstTurnServer::PermissionPrompt | McpFirstTurnServer::Sdk => {
            let stdio = McpTimeoutServer {
                transport: McpServerKind::Stdio,
                timeout: JsValue::Undefined,
            };
            mcp_timeouts(lookup, &stdio).await.startup_ms
        }
        McpFirstTurnServer::Named => math_max(written, NAMED_SERVER_FLOOR_MS),
        McpFirstTurnServer::Other => written,
    })
}

/// "Seconds for which Claude Code uses a discovery-cache entry without refreshing it (default: 900)".
const DEFAULT_TTL_S: f64 = 900.0;
/// "Maximum age, in seconds, of a discovery-cache entry (default: 14400, or 4 hours)".
const DEFAULT_MAX_STALE_S: f64 = 14_400.0;
/// "Claude Code caps the value at 7 days."
const MAX_STALE_CEILING_S: f64 = 7.0 * 24.0 * 3_600.0;
/// "How many refreshes in a row can fail before Claude Code discards the entry ... (default: 1)".
const DEFAULT_STRIKES: f64 = 1.0;

/// The discovery cache's settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpDiscoveryCache {
    /// Whether the cache is on for this session.
    pub enabled: bool,
    /// `MCP_DISCOVERY_CACHE_TTL_S`: seconds an entry is used without a refresh.
    pub ttl_seconds: f64,
    /// `MCP_DISCOVERY_CACHE_MAX_STALE_S`: seconds after which an entry is discarded.
    pub max_stale_seconds: f64,
    /// `MCP_DISCOVERY_CACHE_STRIKES`: failed refreshes in a row that discard an entry.
    pub strikes: f64,
}

/// Whether the rollout enabled the cache for the account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCacheSession {
    /// The rollout's answer.
    pub rolled_out: bool,
}

/// The discovery cache's settings. "The cache is off by default unless a gradual rollout has enabled it
/// for your account. Set to `1` to turn it on, or `0` to keep it off even when the rollout has enabled
/// it." The maximum age is capped at 7 days and the TTL at the maximum age. The releases before v2.1.238,
/// where the cache was on by default and nothing was capped, are not modelled: the kit follows the
/// release it is pinned to.
pub async fn mcp_discovery_cache(
    lookup: &dyn EnvLookup,
    session: McpCacheSession,
) -> McpDiscoveryCache {
    let stale = variable(lookup, "MCP_DISCOVERY_CACHE_MAX_STALE_S").await;
    let max_stale_seconds = math_min(
        positive_whole(stale.as_deref()).unwrap_or(DEFAULT_MAX_STALE_S),
        MAX_STALE_CEILING_S,
    );
    let on = variable(lookup, "MCP_DISCOVERY_CACHE").await;
    let ttl = variable(lookup, "MCP_DISCOVERY_CACHE_TTL_S").await;
    let strikes = variable(lookup, "MCP_DISCOVERY_CACHE_STRIKES").await;
    McpDiscoveryCache {
        enabled: switch_of(on.as_deref()).unwrap_or(session.rolled_out),
        ttl_seconds: math_min(
            positive_whole(ttl.as_deref()).unwrap_or(DEFAULT_TTL_S),
            max_stale_seconds,
        ),
        max_stale_seconds,
        strikes: positive_whole(strikes.as_deref()).unwrap_or(DEFAULT_STRIKES),
    }
}

/// What a start does with a cache entry of a given age.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpCacheUse {
    /// `use`.
    Use,
    /// `use-and-refresh`.
    UseAndRefresh,
    /// `discard`.
    Discard,
}

/// "At a start where the entry is older than [the TTL], Claude Code still uses it but refreshes it in the
/// background. Once the entry is older than [the maximum age], Claude Code discards it" and connects the
/// server at startup. Judgment (spec/judgments.md): an entry exactly as old as a limit is not older.
#[must_use]
pub fn mcp_cache_entry_use(cache: &McpDiscoveryCache, age_seconds: f64) -> McpCacheUse {
    if age_seconds > cache.max_stale_seconds {
        return McpCacheUse::Discard;
    }
    if age_seconds > cache.ttl_seconds {
        McpCacheUse::UseAndRefresh
    } else {
        McpCacheUse::Use
    }
}

/// Whether the entry is discarded once `failures` background refreshes in a row have failed.
#[must_use]
pub fn mcp_cache_discarded_after_failures(cache: &McpDiscoveryCache, failures: f64) -> bool {
    failures >= cache.strikes
}

mod client;

pub use client::{McpClientRuntime, McpClientSettings, mcp_client_settings};

/// Whether a channel server may register as a channel once connected (mcp.md, "Push messages with
/// channels"; spec §9.3 item 5): on the v2 runtime a server that negotiated revision 2026-07-28 does
/// not, since that revision cannot carry channel messages; its tools still work. On the earlier
/// handshake, or on the v1 runtime, it registers as usual.
#[must_use]
pub fn registers_as_channel(runtime: McpClientRuntime, protocol_revision: &str) -> bool {
    !(client::is_v2(runtime)
        && compare_code_units(protocol_revision, NEW_REVISION) != Ordering::Less)
}

crate::registry::exports! {
    "format/mcp-startup" {
        "mcpConnectionBatches" => async ok |lookup: Callback| mcp_connection_batches(&lookup).await;
        "mcpBackgrounding" => async ok |lookup: Callback| mcp_backgrounding(&lookup).await;
        "mcpStartupWaitsFor" => async ok |lookup: Callback, server: McpStartupServer, run: McpStartupRun|
            mcp_startup_waits_for(&lookup, &server, run).await;
        "mcpFirstTurnWaitMs" => async ok |lookup: Callback, server: McpFirstTurnServer|
            mcp_first_turn_wait_ms(&lookup, server).await;
        "mcpDiscoveryCache" => async ok |lookup: Callback, session: McpCacheSession|
            mcp_discovery_cache(&lookup, session).await;
        "mcpCacheEntryUse" => ok |cache: McpDiscoveryCache, age_seconds: f64| mcp_cache_entry_use(&cache, age_seconds);
        "mcpCacheDiscardedAfterFailures" => ok |cache: McpDiscoveryCache, failures: f64|
            mcp_cache_discarded_after_failures(&cache, failures);
        "mcpClientSettings" => async ok |lookup: Callback| mcp_client_settings(&lookup).await;
        "registersAsChannel" => ok |runtime: McpClientRuntime, protocol_revision: String|
            registers_as_channel(runtime, &protocol_revision);
    }
}
