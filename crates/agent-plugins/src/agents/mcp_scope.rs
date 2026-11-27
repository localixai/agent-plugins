//! Port of `src/agents/mcp-scope.ts`, switched.
//!
//! The MCP servers scoped to a subagent (sub-agents.md, "Scope MCP servers to a subagent", "Trust
//! required for inline MCP servers"). The kit resolves; connecting and disconnecting are the host's.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::agent_record::AgentSource;
use crate::js::{JsValue, in_key_order, is_truthy, json_stringify, property_order};
use crate::ports::BoxFuture;
use crate::registry::Port;

/// The restrictions the main session's MCP servers are under, as the host evaluates them.
pub trait McpPolicy: Send + Sync {
    /// Every restriction: `--strict-mcp-config`, `--bare`, managed MCP configuration and the
    /// `allowedMcpServers` / `deniedMcpServers` policies.
    fn allows<'a>(&'a self, server_name: &'a str) -> BoxFuture<'a, bool>;
    /// Only the managed-settings restrictions: "`--strict-mcp-config` doesn't filter servers you pass
    /// inline via `--agents` or the SDK `agents` option, since those are explicit caller input", while
    /// "managed-settings restrictions apply to every subagent regardless of how it is defined".
    fn managed_allows<'a>(&'a self, server_name: &'a str) -> BoxFuture<'a, bool>;
}

/// The host's policy, each method asked of it (its answer read as JavaScript's truthiness).
impl McpPolicy for Port {
    fn allows<'a>(&'a self, server_name: &'a str) -> BoxFuture<'a, bool> {
        let asked = self.method::<JsValue>("allows", vec![server_name.into()]);
        Box::pin(async move { is_truthy(&asked.await) })
    }

    fn managed_allows<'a>(&'a self, server_name: &'a str) -> BoxFuture<'a, bool> {
        let asked = self.method::<JsValue>("managedAllows", vec![server_name.into()]);
        Box::pin(async move { is_truthy(&asked.await) })
    }
}

/// Why a server is skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpSkipReason {
    /// The agent file's folder is not trusted.
    UntrustedFolder,
    /// A policy blocks the server.
    Policy,
}

/// The facts of a folder's trust.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentFolderTrustInput {
    /// The folder whose `.claude/agents/` holds the agent file (an `--add-dir` directory included).
    pub agent_folder: String,
    /// The `projects` object of `~/.claude.json`, keyed by folder path as Claude Code writes it.
    pub projects: IndexMap<String, JsValue>,
    /// The session got trust of its own without the dialog: a `-p` or SDK session's automatic trust for
    /// hooks in settings files. It "doesn't count" and is accepted only so a caller states it.
    #[serde(default)]
    pub automatic_trust: Option<bool>,
    /// The folder is an `--add-dir` directory inside the trusted workspace's repository. Only one
    /// "outside your trusted workspace's repository needs its own trust entry" (judgment: inside it the
    /// workspace's trust stands).
    #[serde(default)]
    pub inside_trusted_workspace: Option<bool>,
}

/// Whether the folder is trusted, and the key that trusts it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AgentFolderTrust {
    /// Whether it is trusted.
    pub trusted: bool,
    /// `projects["<path>"].hasTrustDialogAccepted`: the key a user sets in `~/.claude.json`.
    pub key: String,
}

/// The key of `~/.claude.json` that trusts a folder.
#[must_use]
pub fn trust_key_of(folder: &str) -> String {
    let written = json_stringify(&JsValue::from(folder)).unwrap_or_default();
    format!("projects[{written}].hasTrustDialogAccepted")
}

/// Whether the folder an agent file came from is trusted: its own entry says so ("trust the folder
/// the agent file came from"). A parent folder's entry does not count, and neither does the automatic
/// trust of `-p` or the SDK; the path is compared as written (judgment: the host normalizes it).
#[must_use]
pub fn agent_folder_trust(input: &AgentFolderTrustInput) -> AgentFolderTrust {
    let own = matches!(
        input.projects.get(&input.agent_folder),
        Some(JsValue::Object(entry)) if entry.get("hasTrustDialogAccepted") == Some(&JsValue::Bool(true))
    );
    AgentFolderTrust {
        trusted: own || input.inside_trusted_workspace == Some(true),
        key: trust_key_of(&input.agent_folder),
    }
}

/// The servers an agent's `mcpServers` field gives it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMcpResolution {
    /// Servers named by a string: already configured, they share the parent session's connection.
    pub references: Vec<String>,
    /// Inline definitions, keyed by server name: connected when the subagent starts.
    pub inline: IndexMap<String, JsValue>,
    /// Servers Claude Code skips; it "shows a warning naming the blocked servers".
    pub skipped: Vec<SkippedServer>,
    /// The debug-log lines: "Until then, Claude Code skips every inline server in that agent file and
    /// writes the exact `projects[\"<path>\"].hasTrustDialogAccepted` key". Present only when it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<Vec<String>>,
}

/// A server that is skipped, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkippedServer {
    /// Its name.
    pub name: String,
    /// Why.
    pub reason: McpSkipReason,
}

/// When a scoped server connects and goes away, for the run the agent file starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineServerLifecycle {
    /// `subagent-start` or `session-start`.
    pub connects_at: &'static str,
    /// `subagent-finish` or `session-end`.
    pub disconnects_at: &'static str,
}

/// "Inline servers defined here are connected when the subagent starts ... and disconnected when it
/// finishes"; run as the main session an agent connects them "at startup alongside servers from
/// `.mcp.json` and settings files" (judgment: and they end with the session). A string reference
/// "share\[s\] the parent session's connection", so it has neither.
#[must_use]
pub const fn inline_server_lifecycle(as_main_session: bool) -> InlineServerLifecycle {
    if as_main_session {
        InlineServerLifecycle {
            connects_at: "session-start",
            disconnects_at: "session-end",
        }
    } else {
        InlineServerLifecycle {
            connects_at: "subagent-start",
            disconnects_at: "subagent-finish",
        }
    }
}

/// Whether an inline server of an agent file from this source waits for folder trust: "an inline MCP
/// server from an agent file in your project's `.claude/agents/` directory, or in an `--add-dir`
/// directory's" loads only after the folder is trusted (`project` also stands for `--add-dir`), while
/// `~/.claude/agents/`, `--agents` / the SDK option and managed settings load without it. A plugin
/// agent has no inline servers at all.
const fn inline_needs_trust(source: AgentSource) -> bool {
    match source {
        AgentSource::Managed => false,
        AgentSource::Flag => false,
        AgentSource::Project => true,
        AgentSource::User => false,
        AgentSource::Plugin => false,
    }
}

/// Explicit caller input: only `--agents` / the SDK option, whose inline servers `--strict-mcp-config`
/// does not filter.
const fn is_explicit_input(source: AgentSource) -> bool {
    match source {
        AgentSource::Managed => false,
        AgentSource::Flag => true,
        AgentSource::Project => false,
        AgentSource::User => false,
        AgentSource::Plugin => false,
    }
}

/// Whether the field applies: "Ignored for plugin subagents".
const fn field_applies(source: AgentSource) -> bool {
    match source {
        AgentSource::Managed => true,
        AgentSource::Flag => true,
        AgentSource::Project => true,
        AgentSource::User => true,
        AgentSource::Plugin => false,
    }
}

/// What the resolution reads from the host.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpContext<P> {
    /// Where the agent file came from.
    pub source: AgentSource,
    /// Whether its folder is trusted (`agentFolderTrust`).
    pub folder_trusted: bool,
    /// The key to write to the debug log while an inline server waits for trust (`agentFolderTrust`).
    #[serde(default)]
    pub trust_key: Option<String>,
    /// The policy.
    pub policy: P,
}

async fn inline_skip_reason<P: McpPolicy>(
    name: &str,
    ctx: &McpContext<P>,
) -> Option<McpSkipReason> {
    if inline_needs_trust(ctx.source) && !ctx.folder_trusted {
        return Some(McpSkipReason::UntrustedFolder);
    }
    let allowed = if is_explicit_input(ctx.source) {
        ctx.policy.managed_allows(name).await
    } else {
        ctx.policy.allows(name).await
    };
    (!allowed).then_some(McpSkipReason::Policy)
}

/// The MCP servers an agent's `mcpServers` field gives it. A string is a reference to a configured
/// server, an entry keyed by a name is an inline definition kept as written. A plugin agent has none
/// (its record already drops the field). An inline server from a project or `--add-dir` agent file
/// is skipped until `folderTrusted` (the host's one answer: a parent folder's trust and the automatic
/// trust of `-p` do not count); every other source loads without it. A server the policy blocks is
/// skipped. Connecting and disconnecting the servers is the host's.
pub async fn resolve_agent_mcp_servers<P: McpPolicy>(
    specs: Option<&[JsValue]>,
    ctx: &McpContext<P>,
) -> AgentMcpResolution {
    let mut references: Vec<String> = Vec::new();
    let mut inline: IndexMap<String, JsValue> = IndexMap::new();
    let mut skipped: Vec<SkippedServer> = Vec::new();
    if !field_applies(ctx.source) {
        return AgentMcpResolution {
            references,
            inline,
            skipped,
            debug: None,
        };
    }
    for spec in specs.unwrap_or_default() {
        if let JsValue::String(spec) = spec {
            // A reference needs no trust ("a name that references a server you already configured"); the
            // policy still covers it, since it "covers servers declared in subagent frontmatter" (judgment).
            if ctx.policy.allows(spec).await {
                if !references.contains(spec) {
                    references.push(spec.clone());
                }
            } else {
                skipped.push(SkippedServer {
                    name: spec.clone(),
                    reason: McpSkipReason::Policy,
                });
            }
            continue;
        }
        for (name, config) in entries_of(spec) {
            match inline_skip_reason(&name, ctx).await {
                None => {
                    inline.insert(name, config);
                }
                Some(reason) => skipped.push(SkippedServer { name, reason }),
            }
        }
    }
    let debug = trust_debug(&skipped, ctx.trust_key.as_deref());
    AgentMcpResolution {
        references,
        inline: in_key_order(inline),
        skipped,
        debug,
    }
}

/// `Object.entries(spec)`: an object's members in property order, an array's items by index.
fn entries_of(spec: &JsValue) -> Vec<(String, JsValue)> {
    match spec {
        JsValue::Object(members) => property_order(members)
            .into_iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        JsValue::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item.clone()))
            .collect(),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_) => Vec::new(),
    }
}

const fn needs_trust(reason: McpSkipReason) -> bool {
    match reason {
        McpSkipReason::UntrustedFolder => true,
        McpSkipReason::Policy => false,
    }
}

fn trust_debug(skipped: &[SkippedServer], key: Option<&str>) -> Option<Vec<String>> {
    let key = key?;
    skipped.iter().any(|entry| needs_trust(entry.reason)).then(|| {
        vec![format!(
            "Skipped inline MCP servers of an untrusted folder; set {key} to true in ~/.claude.json"
        )]
    })
}

crate::registry::exports! {
    "agents/mcp-scope" {
        "trustKeyOf" => ok |folder: String| trust_key_of(&folder);
        "agentFolderTrust" => ok |input: AgentFolderTrustInput| agent_folder_trust(&input);
        "inlineServerLifecycle" => ok |as_main_session: bool| inline_server_lifecycle(as_main_session);
        "resolveAgentMcpServers" => async ok |specs: Option<Vec<JsValue>>, ctx: McpContext<Port>|
            resolve_agent_mcp_servers(specs.as_deref(), &ctx).await;
    }
}
