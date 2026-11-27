//! Port of `src/settings/managed/sideload.ts`, switched.
//!
//! `disableSideloadFlags` (settings-reference.md, `disableSideloadFlags`; plugins/troubleshooting.md,
//! "`--plugin-dir is disabled by your organization's managed settings (disableSideloadFlags)`";
//! agent-sdk/typescript.md, `addPlugins`): which command-line sideloads managed settings reject and
//! what a cloud session does instead. The kit reads no command line; the host says what was given.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::registry::Port;
use crate::settings::view::PolicyView;

/// The command-line sideloads.
pub const SIDELOAD_FLAGS: [&str; 4] = ["--plugin-dir", "--plugin-url", "--agents", "--mcp-config"];

/// One server of a `--mcp-config`. `claudeTagSlack` marks a Claude Tag session's Slack tools.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredServer {
    /// Its name.
    pub name: String,
    /// Its type.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    /// A Claude Tag session's Slack tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_tag_slack: Option<bool>,
}

/// What the command line and environment of a session carry.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SideloadFacts {
    /// The `--plugin-dir` values.
    pub plugin_dirs: Vec<String>,
    /// The `--plugin-url` values.
    pub plugin_urls: Vec<String>,
    /// `--agents` was given.
    pub agents: bool,
    /// The servers of `--mcp-config`; empty when the flag was not given.
    pub mcp_servers: Vec<ConfiguredServer>,
    /// The directories `CLAUDE_CODE_PLUGIN_DIRS` names (see `parsePluginDirs`).
    pub plugin_dirs_variable: Vec<String>,
    /// The session is a cloud session.
    pub cloud: bool,
    /// The Agent SDK `plugins` option named plugin folders (plugins/org.md, control matrix).
    #[serde(default)]
    pub sdk_plugins: Option<bool>,
}

/// What the session does under `disableSideloadFlags`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SideloadVerdict {
    /// Startup exits with these lines: one per rejected flag, the Agent SDK `plugins` option or the
    /// variable, then `SIDELOAD_CLOSING`; empty when the session starts.
    pub refusals: Vec<String>,
    /// The `--mcp-config` servers a cloud session starts without.
    pub dropped_servers: Vec<String>,
}

/// "Claude Code still accepts a `--mcp-config` whose servers are all in-process `type: "sdk"` entries."
const IN_PROCESS_TYPES: [&str; 1] = ["sdk"];
fn is_in_process(server: &ConfiguredServer) -> bool {
    server
        .server_type
        .as_deref()
        .is_some_and(|server_type| IN_PROCESS_TYPES.contains(&server_type))
}

/// What each flag has to carry to be rejected: `--mcp-config` only when a server is not in process.
fn rejected_when(flag: &str, facts: &SideloadFacts) -> bool {
    match flag {
        "--plugin-dir" => !facts.plugin_dirs.is_empty(),
        "--plugin-url" => !facts.plugin_urls.is_empty(),
        "--agents" => facts.agents,
        "--mcp-config" => {
            !facts.cloud
                && facts
                    .mcp_servers
                    .iter()
                    .any(|server| !is_in_process(server))
        }
        _ => false,
    }
}

fn refusal_of(flag: &str) -> String {
    format!("{flag} is disabled by your organization's managed settings (disableSideloadFlags)")
}

/// The line after the rejected flags (plugins/troubleshooting.md, "`--plugin-dir is disabled …`").
pub const SIDELOAD_CLOSING: &str = "Plugins, custom agents, and MCP servers can only be loaded from sources your administrator has approved.";

/// The line for the Agent SDK `plugins` option, in the flags' form: its name is the kit's (spec/judgments.md).
const SDK_PLUGINS: &str = "The Agent SDK plugins option";

/// The rejected flags' lines, then the SDK option's and the variable's, then the closing line.
fn refusal_lines(facts: &SideloadFacts) -> Vec<String> {
    let mut lines: Vec<String> = SIDELOAD_FLAGS
        .iter()
        .filter(|flag| rejected_when(flag, facts))
        .map(|flag| refusal_of(flag))
        .collect();
    if facts.sdk_plugins == Some(true) {
        lines.push(refusal_of(SDK_PLUGINS));
    }
    if !facts.plugin_dirs_variable.is_empty() {
        lines.push(format!(
            "{}: unset the variable",
            refusal_of("CLAUDE_CODE_PLUGIN_DIRS")
        ));
    }
    if lines.is_empty() {
        return lines;
    }
    lines.push(SIDELOAD_CLOSING.to_owned());
    lines
}

/// In a cloud session a server survives only as "in-process `type: "sdk"` entries and a Claude Tag session's Slack tools".
fn survives_in_cloud(server: &ConfiguredServer) -> bool {
    is_in_process(server) || server.claude_tag_slack == Some(true)
}

/// Whether managed settings turn the sideload flags off.
async fn flags_disabled(view: &dyn PolicyView) -> bool {
    view.get("disableSideloadFlags").await == Some(JsValue::Bool(true))
}

/// What the session does with its flags and the plugin-folder variable under managed `disableSideloadFlags`.
pub async fn sideload_verdict(view: &dyn PolicyView, facts: &SideloadFacts) -> SideloadVerdict {
    if !flags_disabled(view).await {
        return SideloadVerdict {
            refusals: Vec::new(),
            dropped_servers: Vec::new(),
        };
    }
    let dropped = if facts.cloud {
        facts
            .mcp_servers
            .iter()
            .filter(|server| !survives_in_cloud(server))
            .map(|server| server.name.clone())
            .collect()
    } else {
        Vec::new()
    };
    SideloadVerdict {
        refusals: refusal_lines(facts),
        dropped_servers: dropped,
    }
}

/// The Agent SDK's `addPlugins` option under `disableSideloadFlags`: "that call fails with an error.
/// The session keeps running with the plugins it already had." `None` when the call proceeds.
pub async fn add_plugins_refusal(view: &dyn PolicyView) -> Option<&'static str> {
    flags_disabled(view).await.then_some(
        "reload_plugins add_plugins is disabled by managed settings (disableSideloadFlags)",
    )
}

/// The facts of a mid-session MCP update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct McpUpdateFacts {
    /// The session is a cloud session.
    pub cloud: bool,
}

/// What a mid-session MCP update comes to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct McpUpdateVerdict {
    /// The servers of the update the session applies.
    pub applied: Vec<ConfiguredServer>,
    /// The servers it ignores.
    pub ignored: Vec<ConfiguredServer>,
}

/// Server-delivered mid-session MCP updates, "the path behind cloud session configuration and SDK
/// `setMcpServers()` calls that reach those sessions" (settings-reference.md, `disableSideloadFlags`):
/// under managed `disableSideloadFlags` a cloud session ignores them, except "in-process `type: "sdk"`
/// entries and a Claude Tag session's Slack tools". Any other session applies the update.
pub async fn mid_session_mcp_updates(
    view: &dyn PolicyView,
    facts: &McpUpdateFacts,
    servers: &[ConfiguredServer],
) -> McpUpdateVerdict {
    if !facts.cloud || !flags_disabled(view).await {
        return McpUpdateVerdict {
            applied: servers.to_vec(),
            ignored: Vec::new(),
        };
    }
    McpUpdateVerdict {
        applied: servers
            .iter()
            .filter(|server| survives_in_cloud(server))
            .cloned()
            .collect(),
        ignored: servers
            .iter()
            .filter(|server| !survives_in_cloud(server))
            .cloned()
            .collect(),
    }
}

crate::registry::exports! {
    "settings/managed/sideload" {
        "SIDELOAD_CLOSING" => ok || SIDELOAD_CLOSING;
        "sideloadVerdict" => async ok |view: Port, facts: SideloadFacts|
            sideload_verdict(&view, &facts).await;
        "addPluginsRefusal" => async ok |view: Port| add_plugins_refusal(&view).await;
        "midSessionMcpUpdates" => async ok |view: Port, facts: McpUpdateFacts, servers: Vec<ConfiguredServer>|
            mid_session_mcp_updates(&view, &facts, &servers).await;
    }
}
