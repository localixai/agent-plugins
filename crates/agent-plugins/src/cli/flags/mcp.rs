//! Port of `src/cli/flags/mcp.ts`, switched.
//!
//! `--mcp-config` and `--strict-mcp-config` (cli-reference.md, "CLI flags"; plugins/org.md,
//! `disableSideloadFlags`; headless.md, "Start faster with bare mode"). `--mcp-config` loads "MCP
//! servers from JSON files or strings"; with `-p` Claude Code "waits for still-pending servers to
//! connect before running the first turn, up to the `MCP_TIMEOUT` startup timeout, 30 seconds by
//! default". `--strict-mcp-config` means "only use MCP servers from `--mcp-config`, ignoring all
//! other MCP configurations". `disableSideloadFlags` rejects `--mcp-config` "unless every server in
//! it is an in-process `type: "sdk"` entry". What a server does once loaded is the MCP stream's;
//! this maps the flags to the sources and the wait.

use serde::{Deserialize, Serialize};

use super::sideload::sideload_refusal;
use super::types::{Contribution, Customization, FlagContext, McpConfigSource, refuse};
use super::values::words_of;
use crate::format::env_number::{positive_whole_env, whole_env};
use crate::js::json_parse::json_parse;
use crate::js::{JsObject, JsValue, property_order, trim_start};

/// "30 seconds by default."
pub const DEFAULT_MCP_TIMEOUT_MS: f64 = 30_000.0;

fn is_inline(value: &str) -> bool {
    trim_start(value).starts_with('{')
}

/// A value that starts with `{` is the JSON itself and stays whole; any other holds one or more
/// file paths, "space-separated" (cli-reference.md, `--mcp-config`).
fn sources_of(value: &str) -> Vec<McpConfigSource> {
    if is_inline(value) {
        vec![McpConfigSource::Inline {
            text: value.to_owned(),
        }]
    } else {
        words_of(value)
            .into_iter()
            .map(|path| McpConfigSource::File { path })
            .collect()
    }
}

fn text_of<'a>(source: &'a McpConfigSource, context: &'a FlagContext) -> Option<&'a str> {
    match source {
        McpConfigSource::Inline { text } => Some(text),
        McpConfigSource::File { path } => {
            context.facts.files.get(path).map(|file| file.text.as_str())
        }
    }
}

fn is_in_process_server(server: &JsValue) -> bool {
    server.get("type").and_then(JsValue::as_str) == Some("sdk")
}

/// The `mcpServers` object of a config, by name; a config that cannot be read has none.
fn servers_in(text: Option<&str>) -> Option<JsObject> {
    let parsed = json_parse(text?).ok()?;
    parsed.get("mcpServers")?.as_object().cloned()
}

/// Whether every server of the config is `type: "sdk"`; a config that cannot be read is not.
fn only_sdk_servers(text: Option<&str>) -> bool {
    servers_in(text).is_some_and(|servers| servers.values().all(is_in_process_server))
}

/// The `type`s of a remote server: "a remote (`http` or `sse`) server" (mcp.md, "Server status
/// detail").
const REMOTE_TYPES: [&str; 2] = ["http", "sse"];

fn is_remote_server(server: &JsValue) -> bool {
    server
        .get("type")
        .and_then(JsValue::as_str)
        .is_some_and(|kind| REMOTE_TYPES.contains(&kind))
}

fn env<'a>(context: &'a FlagContext, name: &str) -> Option<&'a str> {
    context.facts.env.get(name).and_then(Option::as_deref)
}

/// `MCP_TIMEOUT` is in milliseconds, a positive whole number read as every numeric variable is
/// (spec §3.6.1 item 2, J3.17: `2e3`, `64_000`); anything else is the default.
fn timeout_ms(context: &FlagContext) -> f64 {
    positive_whole_env(env(context, "MCP_TIMEOUT")).unwrap_or(DEFAULT_MCP_TIMEOUT_MS)
}

/// "In a self-hosted environment, a shorter wait applies instead" (cli-reference.md,
/// `--mcp-config`): "the first turn waits up to 2 seconds for stdio servers that are still
/// connecting", and `CLAUDE_CODE_MCP_STARTUP_WAIT_MS` "changes only how long the wait lasts"
/// (self-hosted-environments-configuration.md, "Wait for MCP servers before the first turn").
const SELF_HOSTED_MCP_WAIT_MS: f64 = 2_000.0;

fn self_hosted_wait_ms(context: &FlagContext) -> f64 {
    // J3.17: a whole number (`2e3`, `64_000`); `0` skips the wait.
    whole_env(env(context, "CLAUDE_CODE_MCP_STARTUP_WAIT_MS")).unwrap_or(SELF_HOSTED_MCP_WAIT_MS)
}

fn wait_ms(context: &FlagContext) -> f64 {
    if context.facts.self_hosted == Some(true) {
        self_hosted_wait_ms(context)
    } else {
        timeout_ms(context)
    }
}

/// The servers the first turn does not wait for: "a server with a cached tool list skips the wait
/// and connects on first use" (only a remote one has a cached tool list: agent-sdk/mcp.md,
/// "Connection timing"), and in a self-hosted environment the wait covers stdio servers only,
/// HTTP and SSE ones connecting in the background.
fn wait_skips(sources: &[McpConfigSource], context: &FlagContext) -> Vec<String> {
    let cached = context
        .facts
        .cached_tool_lists
        .as_deref()
        .unwrap_or_default();
    let self_hosted = context.facts.self_hosted == Some(true);
    sources
        .iter()
        .flat_map(|source| {
            let servers = servers_in(text_of(source, context)).unwrap_or_default();
            property_order(&servers)
                .into_iter()
                .filter(|(name, server)| {
                    is_remote_server(server) && (self_hosted || cached.contains(name))
                })
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The contribution of `--mcp-config`.
#[must_use]
pub fn mcp_config(values: &[String], context: &FlagContext) -> Contribution {
    let sources: Vec<McpConfigSource> = values.iter().flat_map(|value| sources_of(value)).collect();
    if let Some(refusal) = sideload_refusal("--mcp-config", &context.facts)
        && !sources
            .iter()
            .all(|source| only_sdk_servers(text_of(source, context)))
    {
        return refuse(vec![refusal]);
    }
    let print = context.facts.print;
    Contribution {
        mcp_wait_ms: print.then(|| wait_ms(context)),
        mcp_wait_skips: print.then(|| wait_skips(&sources, context)),
        mcp_configs: Some(sources),
        ..Contribution::default()
    }
}

/// The contribution of `--strict-mcp-config`.
#[must_use]
pub fn strict_mcp_config() -> Contribution {
    Contribution {
        strict_mcp_config: Some(true),
        ..Contribution::default()
    }
}

/// The places MCP servers are read from.
pub const MCP_READ_SOURCES: [&str; 5] = ["user", "project", "local", "plugins", "command-line"];

/// What `mcpSourcesRead` reads of the plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPlanView {
    /// `--strict-mcp-config`.
    pub strict_mcp_config: bool,
    /// The customizations the session leaves out.
    pub skipped: Vec<Customization>,
}

/// The sources a session reads servers from: `--strict-mcp-config` and `--bare` leave the command
/// line's alone ("only servers supplied on the command line connect"). How a managed MCP file
/// combines with `--strict-mcp-config` is the MCP stream's (managed-mcp.md).
#[must_use]
pub fn mcp_sources_read(plan: &McpPlanView) -> Vec<&'static str> {
    let command_line_only =
        plan.strict_mcp_config || plan.skipped.contains(&Customization::McpServers);
    if command_line_only {
        vec!["command-line"]
    } else {
        MCP_READ_SOURCES.to_vec()
    }
}

crate::registry::exports! {
    "cli/flags/mcp" {
        "DEFAULT_MCP_TIMEOUT_MS" => ok || DEFAULT_MCP_TIMEOUT_MS;
        "mcpConfig" => ok |values: Vec<String>, context: FlagContext| mcp_config(&values, &context);
        "strictMcpConfig" => ok || Contribution { ..strict_mcp_config() };
        "MCP_READ_SOURCES" => ok || MCP_READ_SOURCES;
        "mcpSourcesRead" => ok |plan: McpPlanView| mcp_sources_read(&plan);
    }
}
