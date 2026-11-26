//! Port of `src/loading/resolve/headers-helper-origins.ts`, switched.
//!
//! Where an MCP server was configured, as far as its `headersHelper` is concerned (mcp.md, "Where
//! the helper runs", "Which variables a helper can read" and "Trust a folder before its
//! headersHelper runs"). The first six are the scopes of `settings/mcp` (`McpScope`, a plugin and a
//! connector included), so a ranked candidate's `scope` is one of these; the rest are the places the
//! three tables name that are not scopes: the SDK, `--mcp-config` and the five sources of an inline
//! server in an agent file.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::config_directory::{CONFIG_DIR_VARIABLE, JoinPath, config_directory_of};
use crate::ports::locations::Locations;
use crate::registry::Port;

/// Every place, in the TypeScript's order.
pub const HEADERS_HELPER_ORIGINS: [HeadersHelperOrigin; 13] = [
    HeadersHelperOrigin::Plugin,
    HeadersHelperOrigin::Project,
    HeadersHelperOrigin::Local,
    HeadersHelperOrigin::User,
    HeadersHelperOrigin::Managed,
    HeadersHelperOrigin::Connector,
    HeadersHelperOrigin::Sdk,
    HeadersHelperOrigin::McpConfig,
    HeadersHelperOrigin::AgentProject,
    HeadersHelperOrigin::AgentAddDir,
    HeadersHelperOrigin::AgentUser,
    HeadersHelperOrigin::AgentManaged,
    HeadersHelperOrigin::AgentFlag,
];

/// One of `HEADERS_HELPER_ORIGINS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeadersHelperOrigin {
    /// `plugin`.
    Plugin,
    /// `project`.
    Project,
    /// `local`.
    Local,
    /// `user`.
    User,
    /// `managed`.
    Managed,
    /// `connector`.
    Connector,
    /// `sdk`.
    Sdk,
    /// `mcp-config`.
    McpConfig,
    /// `agent-project`.
    AgentProject,
    /// `agent-add-dir`.
    AgentAddDir,
    /// `agent-user`.
    AgentUser,
    /// `agent-managed`.
    AgentManaged,
    /// `agent-flag`.
    AgentFlag,
}

/// The kinds of working directory the "Where the helper runs" table names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryKind {
    /// The plugin's root.
    PluginRoot,
    /// The project directory the server is declared in.
    ProjectDirectory,
    /// The session's primary working directory.
    PrimaryDirectory,
    /// The configuration directory.
    ConfigDirectory,
}

/// Who must be trusted before anything runs: nobody, the helper alone, or the server's loading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustGate {
    /// Nobody.
    None,
    /// The helper alone.
    Helper,
    /// The server's loading.
    Load,
}

/// What the three tables say of one place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginRule {
    /// Where the helper runs.
    pub directory: DirectoryKind,
    /// "Removed" in the "Which variables a helper can read" lists.
    pub removes_credentials: bool,
    /// Who must be trusted first.
    pub gate: TrustGate,
    /// A plugin-provided server has a plugin root: the helper gets `CLAUDE_PLUGIN_ROOT`.
    pub provides_plugin_root: bool,
}

/// The three tables of mcp.md in one row per place. Judgments (spec/judgments.md): an inline server
/// of an agent from managed settings or `--agents` is "from outside your project", so it runs in the
/// configuration directory; the docs list both only under "Not removed".
#[must_use]
pub const fn origin_rule(origin: HeadersHelperOrigin) -> OriginRule {
    match origin {
        HeadersHelperOrigin::Plugin => OriginRule {
            directory: DirectoryKind::PluginRoot,
            removes_credentials: true,
            gate: TrustGate::None,
            provides_plugin_root: true,
        },
        HeadersHelperOrigin::Project => OriginRule {
            directory: DirectoryKind::ProjectDirectory,
            removes_credentials: true,
            gate: TrustGate::Helper,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::Local => OriginRule {
            directory: DirectoryKind::ProjectDirectory,
            removes_credentials: false,
            gate: TrustGate::Helper,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::User => OriginRule {
            directory: DirectoryKind::ConfigDirectory,
            removes_credentials: false,
            gate: TrustGate::None,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::Managed => OriginRule {
            directory: DirectoryKind::ConfigDirectory,
            removes_credentials: false,
            gate: TrustGate::None,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::Connector => OriginRule {
            directory: DirectoryKind::ConfigDirectory,
            removes_credentials: false,
            gate: TrustGate::None,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::Sdk => OriginRule {
            directory: DirectoryKind::PrimaryDirectory,
            removes_credentials: false,
            gate: TrustGate::None,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::McpConfig => OriginRule {
            directory: DirectoryKind::PrimaryDirectory,
            removes_credentials: false,
            gate: TrustGate::None,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::AgentProject => OriginRule {
            directory: DirectoryKind::PrimaryDirectory,
            removes_credentials: true,
            gate: TrustGate::Load,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::AgentAddDir => OriginRule {
            directory: DirectoryKind::ConfigDirectory,
            removes_credentials: true,
            gate: TrustGate::Load,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::AgentUser => OriginRule {
            directory: DirectoryKind::ConfigDirectory,
            removes_credentials: false,
            gate: TrustGate::None,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::AgentManaged => OriginRule {
            directory: DirectoryKind::ConfigDirectory,
            removes_credentials: false,
            gate: TrustGate::None,
            provides_plugin_root: false,
        },
        HeadersHelperOrigin::AgentFlag => OriginRule {
            directory: DirectoryKind::ConfigDirectory,
            removes_credentials: false,
            gate: TrustGate::None,
            provides_plugin_root: false,
        },
    }
}

/// What the plan reads from the host: the environment it inherits and the directories it picks from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperContext<L> {
    /// The environment the helper starts from, before any removal.
    pub environment: IndexMap<String, String>,
    /// Its `homeDir` and `joinPath`.
    pub locations: L,
    /// The project directory a project or local server is declared in.
    pub project_directory: String,
    /// The session's primary working directory (it moves with `/cd`).
    pub primary_directory: String,
    /// The plugin's root; only a plugin-provided server has one.
    #[serde(default)]
    pub plugin_root: Option<String>,
}

/// "Your configuration directory, `~/.claude` unless you set `CLAUDE_CONFIG_DIR`."
pub async fn config_directory<L: Locations>(context: &HelperContext<L>) -> String {
    let home = context.locations.home_dir().await;
    let config_dir = context.environment.get(CONFIG_DIR_VARIABLE);
    config_directory_of(
        &home,
        config_dir.map(String::as_str),
        &JoinPath(&context.locations),
    )
    .await
}

/// The directory of each kind (`DIRECTORIES[kind](context)`).
pub async fn directory_of<L: Locations>(
    kind: DirectoryKind,
    context: &HelperContext<L>,
) -> Option<String> {
    match kind {
        DirectoryKind::PluginRoot => context.plugin_root.clone(),
        DirectoryKind::ProjectDirectory => Some(context.project_directory.clone()),
        DirectoryKind::PrimaryDirectory => Some(context.primary_directory.clone()),
        DirectoryKind::ConfigDirectory => Some(config_directory(context).await),
    }
}

crate::registry::exports! {
    "loading/resolve/headers-helper-origins" {
        "HEADERS_HELPER_ORIGINS" => ok || HEADERS_HELPER_ORIGINS;
        "configDirectory" => async ok |context: HelperContext<Port>| config_directory(&context).await;
    }
}
