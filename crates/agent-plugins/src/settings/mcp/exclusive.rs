//! Port of `src/settings/mcp/exclusive.ts`, switched.
//!
//! Which servers load under the organization's MCP configuration, and what Claude Code says and does
//! about the ones that do not (managed-mcp.md, "Exclusive control with managed-mcp.json").
//!
//! A planned server's source is found by data: a managed candidate is the file's when the file
//! defines its name. The file's servers come first among the managed ones and the ranking keeps one
//! managed server per name, so a `managedMcpServers` entry of a name the file defines never loads;
//! the TypeScript compared the server objects by identity, which cannot cross. A candidate whose
//! scope has no gate (the type allows none: `managed` comes from the file and the provided servers)
//! is left out.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::js::value::{JsObject, to_js_value};
use crate::registry::{Own, Port};
use crate::settings::mcp::allow_deny::{McpPolicySubject, McpServerPolicy};
use crate::settings::mcp::load_settings::McpLoadSettings;
use crate::settings::mcp::managed_file::ManagedMcpFile;
use crate::settings::mcp::scope::{HiddenMcpServer, RankOptions, rank_mcp_servers};
use crate::settings::mcp::toggles::{ProjectToggles, apply_toggles};
use crate::settings::undefined_if_none;

/// `claude mcp add` while `managed-mcp.json` is deployed (managed-mcp.md, "Validate the configuration").
pub const EXCLUSIVE_ADD_ERROR: &str = "Cannot add MCP server: enterprise MCP configuration is active and has exclusive control over MCP servers";

/// The startup exit of `--mcp-config` on a workstation (managed-mcp.md, "Servers passed with --mcp-config").
pub const MCP_CONFIG_EXIT: &str =
    "You cannot dynamically configure MCP servers when an enterprise MCP config is present";

/// The startup exit of `--strict-mcp-config`. The page says Claude Code "exits at startup" without
/// quoting it (spec/judgments.md).
pub const STRICT_MCP_CONFIG_EXIT: &str =
    "You cannot use --strict-mcp-config when an enterprise MCP config is present";

/// The startup exit of `claude --chrome` (managed-mcp.md, "How restrictions appear to users").
pub const CHROME_BLOCKED_ERROR: &str = "Claude in Chrome is blocked by your organization's managed MCP configuration (managed-mcp.json). An administrator can allow it with allowClaudeInChromeWithManagedMcp in device policy.";

/// The name of the built-in Claude in Chrome server, which a `deniedMcpServers` entry can name.
const CHROME_SERVER_NAME: &str = "claude-in-chrome";

/// A server and its name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedMcpServer {
    /// The name.
    pub name: String,
    /// The server (`PluginMcpServer`).
    pub server: JsValue,
}

/// Whether the user could run Claude in Chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromeWish {
    /// They ran `claude --chrome`.
    pub flag: bool,
    /// They turned it on by default.
    pub by_default: bool,
}

/// What the session is.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSessionFacts {
    /// A cloud session: the host delivers connectors and other servers through `--mcp-config`.
    pub cloud: bool,
    /// The servers `--mcp-config` passes (in a cloud session, the ones the cloud host delivers).
    #[serde(default)]
    pub mcp_config: Option<Vec<NamedMcpServer>>,
    /// `--strict-mcp-config`.
    #[serde(default)]
    pub strict_mcp_config: Option<bool>,
    /// Set only for a user who could run Claude in Chrome.
    #[serde(default)]
    pub chrome: Option<ChromeWish>,
}

/// The allow and deny lists over one server: the core's own (`createMcpServerPolicy`), or the
/// host's object with a `check` method.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ServerPolicy {
    /// The core's policy, given back.
    Core(Own<McpServerPolicy>),
    /// The host's object.
    Host(Port),
}

impl ServerPolicy {
    /// `policy.check(subject)`: the verdict, its `allowed` member read.
    async fn check(&self, subject: &JsValue) -> JsValue {
        match self {
            Self::Core(policy) => match McpPolicySubject::deserialize(subject.clone()) {
                Ok(subject) => to_js_value(&policy.check(&subject).await).unwrap_or_default(),
                Err(_) => JsValue::Undefined,
            },
            Self::Host(port) => port.method("check", vec![subject.clone()]).await,
        }
    }
}

/// What the plan reads.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLoadInput {
    /// `readManagedMcpFile(...).file`: none when no file is deployed or it cannot be read.
    #[serde(default)]
    pub managed_file: Option<ManagedMcpFile>,
    /// `readManagedMcpServers(...).servers` (`McpCandidate`s).
    pub managed_servers: Vec<JsValue>,
    /// The servers of the other scopes (`ConfiguredCandidate`s).
    pub candidates: Vec<JsValue>,
    /// The switches.
    pub settings: McpLoadSettings,
    /// The lists.
    pub policy: ServerPolicy,
    /// The session.
    pub session: McpSessionFacts,
    /// The project's `/mcp` toggles; a disabled server stays in the plan, marked.
    #[serde(default)]
    pub toggles: Option<ProjectToggles>,
}

/// Where a planned server comes from; the two organization sources are told apart for the lists.
pub const PLAN_SOURCES: [&str; 8] = [
    "managedFile",
    "managedSettings",
    "local",
    "project",
    "user",
    "plugin",
    "connector",
    "flag",
];

/// What loads and what Claude Code says.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLoadPlan {
    /// Why Claude Code exits at startup, or `undefined` when the session starts.
    #[serde(serialize_with = "undefined_if_none")]
    pub exit: Option<&'static str>,
    /// A readable `managed-mcp.json` is deployed.
    pub exclusive: bool,
    /// What loads, highest scope first (servers of `--mcp-config` last): `PlannedServer`s.
    pub servers: Vec<JsValue>,
    /// What `allowedMcpServers` and `deniedMcpServers` stop: `BlockedServer`s.
    pub blocked: Vec<JsValue>,
    /// What a server above displaced.
    pub hidden: Vec<HiddenMcpServer>,
    /// The servers a cloud host delivered that exclusive control skipped.
    pub skipped: Vec<String>,
    /// What Claude Code writes to stderr.
    pub warnings: Vec<String>,
    /// What `claude mcp add` says while the file is deployed.
    #[serde(serialize_with = "undefined_if_none")]
    pub add_error: Option<&'static str>,
    /// The built-in Claude in Chrome server runs.
    pub chrome: bool,
}

/// The origin the lists read for a source.
fn origin_of(source: &str) -> &str {
    match source {
        "managedFile" | "managedSettings" => source,
        _ => "configured",
    }
}

/// What decides whether a scope's servers load.
#[derive(Clone, Copy)]
struct Gate {
    exclusive: bool,
    plugin_only_mcp: bool,
    connectors: bool,
}

const fn open(gate: Gate) -> bool {
    !gate.exclusive && !gate.plugin_only_mcp
}

/// Exclusive control "includes plugin-provided servers" in what users cannot use; the connectors Claude
/// Code fetches load unless disabled, and under the file only when `allowAllClaudeAiMcps` allows them;
/// `strictPluginOnlyCustomization` with `mcp` closes `~/.claude.json` and `.mcp.json` (managed-mcp.md,
/// "Choose a pattern").
const SCOPE_GATES: [(&str, fn(Gate) -> bool); 5] = [
    ("local", open),
    ("project", open),
    ("user", open),
    ("plugin", |gate| !gate.exclusive),
    ("connector", |gate| gate.connectors),
];

type Exit = fn(&McpLoadInput, bool) -> Option<&'static str>;

/// The startup exits, in the order the page lists the cases (`--mcp-config`, then `--strict-mcp-config`,
/// then Chrome); the first that applies is the one. The page does not say which message wins when
/// several apply (spec/judgments.md).
const EXITS: [Exit; 3] = [
    |input, exclusive| {
        let flagged = input
            .session
            .mcp_config
            .as_ref()
            .is_some_and(|servers| !servers.is_empty());
        (exclusive && !input.session.cloud && flagged).then_some(MCP_CONFIG_EXIT)
    },
    |input, exclusive| {
        (exclusive && input.session.strict_mcp_config == Some(true))
            .then_some(STRICT_MCP_CONFIG_EXIT)
    },
    |input, exclusive| {
        let flag = input.session.chrome.is_some_and(|chrome| chrome.flag);
        (exclusive && flag && !input.settings.allow_claude_in_chrome_with_managed_mcp)
            .then_some(CHROME_BLOCKED_ERROR)
    },
];

fn text<'a>(value: &'a JsValue, key: &str) -> &'a str {
    value.get(key).and_then(JsValue::as_str).unwrap_or_default()
}

/// A managed candidate is the file's when the file defines its name (the module comment says why).
fn source_of(candidate: &JsValue, file: Option<&ManagedMcpFile>) -> String {
    let scope = text(candidate, "scope");
    if scope != "managed" {
        return scope.to_owned();
    }
    let in_file = file.is_some_and(|file| file.servers.contains_key(text(candidate, "name")));
    (if in_file {
        "managedFile"
    } else {
        "managedSettings"
    })
    .to_owned()
}

fn object(members: Vec<(&str, JsValue)>) -> JsValue {
    JsValue::Object(
        members
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

async fn chrome_runs(input: &McpLoadInput, exclusive: bool) -> bool {
    if !input
        .session
        .chrome
        .is_some_and(|wants| wants.flag || wants.by_default)
    {
        return false;
    }
    if exclusive && !input.settings.allow_claude_in_chrome_with_managed_mcp {
        return false;
    }
    let stand_in = object(vec![("command", JsValue::from(CHROME_SERVER_NAME))]);
    let subject = object(vec![
        ("name", JsValue::from(CHROME_SERVER_NAME)),
        ("server", stand_in),
        ("origin", JsValue::from("builtIn")),
    ]);
    input.policy.check(&subject).await.get("allowed") == Some(&JsValue::Bool(true))
}

const fn empty() -> McpLoadPlan {
    McpLoadPlan {
        exit: None,
        exclusive: false,
        servers: Vec::new(),
        blocked: Vec::new(),
        hidden: Vec::new(),
        skipped: Vec::new(),
        warnings: Vec::new(),
        add_error: None,
        chrome: false,
    }
}

/// The candidates of the plan: the file's, the provided ones and the scopes that stay open. `/mcp`
/// toggles apply to the provided servers and the scopes that stay open; the page documents them for
/// provided servers only, and a file server is always on (spec/judgments.md).
fn candidates_of(input: &McpLoadInput, exclusive: bool) -> Vec<JsValue> {
    if input.session.strict_mcp_config == Some(true) {
        return Vec::new();
    }
    let settings = &input.settings;
    let gate = Gate {
        exclusive,
        plugin_only_mcp: settings.plugin_only_mcp,
        connectors: !settings.disable_claude_ai_connectors
            && (!exclusive || (settings.allow_all_claude_ai_mcps && !input.session.cloud)),
    };
    let file_servers = input.managed_file.as_ref().map(|file| &file.servers);
    let from_file = file_servers.into_iter().flatten().map(|(name, server)| {
        object(vec![
            ("scope", JsValue::from("managed")),
            ("name", JsValue::from(name.as_str())),
            ("server", server.clone()),
            ("enabled", JsValue::Bool(true)),
        ])
    });
    let gated = |each: &&JsValue| {
        let scope = text(each, "scope");
        SCOPE_GATES
            .iter()
            .any(|(name, open)| *name == scope && open(gate))
    };
    let rest = input.candidates.iter().filter(gated);
    let provided = input.managed_servers.iter().chain(rest).cloned().collect();
    let toggles = input.toggles.clone().unwrap_or_default();
    from_file.chain(apply_toggles(provided, &toggles)).collect()
}

/// A planned server: its name, server, source, whether it is on, and its origin.
fn planned(name: JsValue, server: JsValue, source: &str, enabled: bool) -> JsValue {
    object(vec![
        ("name", name),
        ("server", server),
        ("source", JsValue::from(source)),
        ("enabled", JsValue::Bool(enabled)),
        ("origin", JsValue::from(origin_of(source))),
    ])
}

fn member(value: &JsValue, key: &str) -> JsValue {
    value.get(key).cloned().unwrap_or_default()
}

/// The servers that load, and the ones the lists stop.
async fn checked(input: &McpLoadInput, planned: Vec<JsValue>) -> (Vec<JsValue>, Vec<JsValue>) {
    let (mut servers, mut blocked) = (Vec::new(), Vec::new());
    for each in planned {
        let verdict = input.policy.check(&each).await;
        if verdict.get("allowed") == Some(&JsValue::Bool(true)) {
            servers.push(each);
            continue;
        }
        let mut entry: JsObject = ["name", "server", "source"]
            .into_iter()
            .map(|key| (key.to_owned(), member(&each, key)))
            .collect();
        if let JsValue::Object(members) = verdict {
            entry.extend(members);
        }
        blocked.push(JsValue::Object(entry));
    }
    (servers, blocked)
}

/// Which servers load, and what Claude Code says and does about the ones that do not (managed-mcp.md,
/// "Exclusive control with managed-mcp.json" and the sections under it). With a `managed-mcp.json` that
/// can be read and parsed, only its servers and the provided ones load; the lists (`deniedMcpServers`
/// always, `allowedMcpServers` for what is not the organization's own) filter the result. Without it
/// the scopes rank as usual and the flags keep their meaning: a `--mcp-config` server replaces a
/// provided one of its name and meets the allowlist, `--strict-mcp-config` leaves everything else out.
/// The built-in in-process servers of the app that started the session never pass through here.
pub async fn mcp_load_plan(input: &McpLoadInput) -> McpLoadPlan {
    let exclusive = input.managed_file.is_some();
    let add_error = exclusive.then_some(EXCLUSIVE_ADD_ERROR);
    if let Some(exit) = EXITS.iter().find_map(|each| each(input, exclusive)) {
        return McpLoadPlan {
            exit: Some(exit),
            exclusive,
            add_error,
            ..empty()
        };
    }
    let given = input.session.mcp_config.clone().unwrap_or_default();
    let (flagged, skipped): (Vec<NamedMcpServer>, Vec<String>) = if exclusive {
        (
            Vec::new(),
            given.into_iter().map(|each| each.name).collect(),
        )
    } else {
        (given, Vec::new())
    };
    let flag_names = |name: &str| flagged.iter().any(|each| each.name == name);
    let kept: Vec<JsValue> = candidates_of(input, exclusive)
        .into_iter()
        .filter(|each| text(each, "scope") != "managed" || !flag_names(text(each, "name")))
        .collect();
    let ranking = rank_mcp_servers(&kept, &RankOptions::default());
    let file = input.managed_file.as_ref();
    let loaded = ranking.loaded.iter().map(|each| {
        let enabled = each.get("enabled") != Some(&JsValue::Bool(false));
        planned(
            member(each, "name"),
            member(each, "server"),
            &source_of(each, file),
            enabled,
        )
    });
    let flags = flagged.iter().map(|each| {
        planned(
            JsValue::from(each.name.as_str()),
            each.server.clone(),
            "flag",
            true,
        )
    });
    let (servers, blocked) = checked(input, loaded.chain(flags).collect()).await;
    let warnings = if skipped.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "Skipped MCP servers because an enterprise MCP config is present: {}",
            skipped.join(", ")
        )]
    };
    McpLoadPlan {
        exclusive,
        add_error,
        servers,
        blocked,
        hidden: ranking.hidden,
        skipped,
        warnings,
        chrome: chrome_runs(input, exclusive).await,
        ..empty()
    }
}

crate::registry::exports! {
    "settings/mcp/exclusive" {
        "EXCLUSIVE_ADD_ERROR" => ok || EXCLUSIVE_ADD_ERROR;
        "MCP_CONFIG_EXIT" => ok || MCP_CONFIG_EXIT;
        "STRICT_MCP_CONFIG_EXIT" => ok || STRICT_MCP_CONFIG_EXIT;
        "CHROME_BLOCKED_ERROR" => ok || CHROME_BLOCKED_ERROR;
        "PLAN_SOURCES" => ok || PLAN_SOURCES;
        "mcpLoadPlan" => async ok |input: McpLoadInput| mcp_load_plan(&input).await;
    }
}
