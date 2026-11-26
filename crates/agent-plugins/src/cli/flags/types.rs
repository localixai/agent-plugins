//! Port of `src/cli/flags/types.ts`, switched.
//!
//! The shapes of the session flags (cli-reference.md, "CLI flags"): what the host parsed from the
//! command line (`FlagArguments`), what it knows about the machine (`FlagFacts`) and what the flags
//! add up to (`SessionPlan`). Every flag is one strategy in `flags/registry.ts` over `FlagName`; the
//! plan is data the host hands to the modules that own each behaviour (settings, loading, agents,
//! rules, mcp), so no decision is made twice.
//!
//! `ChannelEntry` (`src/settings/channels/verdict.ts`) and `PolicyLayer` (`src/settings/layers.ts`)
//! are not in the core yet: a member of those types is a [`JsValue`] here, which is what crosses the
//! boundary for them; `PolicyScope` and the SDK's `PermissionMode` are strings.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::js::{JsObject, JsValue};
use crate::registry::Rest;

/// One of the flags the session plan reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlagName {
    /// `--plugin-dir`.
    #[serde(rename = "--plugin-dir")]
    PluginDir,
    /// `--plugin-url`.
    #[serde(rename = "--plugin-url")]
    PluginUrl,
    /// `--bare`.
    #[serde(rename = "--bare")]
    Bare,
    /// `--safe-mode`.
    #[serde(rename = "--safe-mode")]
    SafeMode,
    /// `--restricted`.
    #[serde(rename = "--restricted")]
    Restricted,
    /// `--setting-sources`.
    #[serde(rename = "--setting-sources")]
    SettingSources,
    /// `--settings`.
    #[serde(rename = "--settings")]
    Settings,
    /// `--disable-slash-commands`.
    #[serde(rename = "--disable-slash-commands")]
    DisableSlashCommands,
    /// `--add-dir`.
    #[serde(rename = "--add-dir")]
    AddDir,
    /// `--agent`.
    #[serde(rename = "--agent")]
    Agent,
    /// `--agents`.
    #[serde(rename = "--agents")]
    Agents,
    /// `--append-subagent-system-prompt`.
    #[serde(rename = "--append-subagent-system-prompt")]
    AppendSubagentSystemPrompt,
    /// `--append-subagent-system-prompt-file`.
    #[serde(rename = "--append-subagent-system-prompt-file")]
    AppendSubagentSystemPromptFile,
    /// `--forward-subagent-text`.
    #[serde(rename = "--forward-subagent-text")]
    ForwardSubagentText,
    /// `--channels`.
    #[serde(rename = "--channels")]
    Channels,
    /// `--dangerously-load-development-channels`.
    #[serde(rename = "--dangerously-load-development-channels")]
    DangerouslyLoadDevelopmentChannels,
    /// `--mcp-config`.
    #[serde(rename = "--mcp-config")]
    McpConfig,
    /// `--strict-mcp-config`.
    #[serde(rename = "--strict-mcp-config")]
    StrictMcpConfig,
    /// `--permission-mode`.
    #[serde(rename = "--permission-mode")]
    PermissionMode,
    /// `--tools`.
    #[serde(rename = "--tools")]
    Tools,
}

/// Every flag, in the order of the table.
pub const FLAG_NAMES: [FlagName; 20] = [
    FlagName::PluginDir,
    FlagName::PluginUrl,
    FlagName::Bare,
    FlagName::SafeMode,
    FlagName::Restricted,
    FlagName::SettingSources,
    FlagName::Settings,
    FlagName::DisableSlashCommands,
    FlagName::AddDir,
    FlagName::Agent,
    FlagName::Agents,
    FlagName::AppendSubagentSystemPrompt,
    FlagName::AppendSubagentSystemPromptFile,
    FlagName::ForwardSubagentText,
    FlagName::Channels,
    FlagName::DangerouslyLoadDevelopmentChannels,
    FlagName::McpConfig,
    FlagName::StrictMcpConfig,
    FlagName::PermissionMode,
    FlagName::Tools,
];

/// What each flag was given: a repeatable flag its values in order, a switch `true`, the rest the text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's record of the flags: independent yes-or-no facts, as the interface has them"
)]
pub struct FlagArguments {
    /// `--plugin-dir`.
    #[serde(rename = "--plugin-dir")]
    pub plugin_dir: Vec<String>,
    /// `--plugin-url`.
    #[serde(rename = "--plugin-url")]
    pub plugin_url: Vec<String>,
    /// `--bare`.
    #[serde(rename = "--bare")]
    pub bare: bool,
    /// `--safe-mode`.
    #[serde(rename = "--safe-mode")]
    pub safe_mode: bool,
    /// `--restricted`.
    #[serde(rename = "--restricted")]
    pub restricted: bool,
    /// `--setting-sources`.
    #[serde(rename = "--setting-sources")]
    pub setting_sources: String,
    /// `--settings`.
    #[serde(rename = "--settings")]
    pub settings: String,
    /// `--disable-slash-commands`.
    #[serde(rename = "--disable-slash-commands")]
    pub disable_slash_commands: bool,
    /// `--add-dir`.
    #[serde(rename = "--add-dir")]
    pub add_dir: Vec<String>,
    /// `--agent`.
    #[serde(rename = "--agent")]
    pub agent: String,
    /// `--agents`.
    #[serde(rename = "--agents")]
    pub agents: String,
    /// `--append-subagent-system-prompt`.
    #[serde(rename = "--append-subagent-system-prompt")]
    pub append_subagent_system_prompt: String,
    /// `--append-subagent-system-prompt-file`.
    #[serde(rename = "--append-subagent-system-prompt-file")]
    pub append_subagent_system_prompt_file: String,
    /// `--forward-subagent-text`.
    #[serde(rename = "--forward-subagent-text")]
    pub forward_subagent_text: bool,
    /// `--channels`.
    #[serde(rename = "--channels")]
    pub channels: Vec<String>,
    /// `--dangerously-load-development-channels`.
    #[serde(rename = "--dangerously-load-development-channels")]
    pub dangerously_load_development_channels: Vec<String>,
    /// `--mcp-config`.
    #[serde(rename = "--mcp-config")]
    pub mcp_config: Vec<String>,
    /// `--strict-mcp-config`.
    #[serde(rename = "--strict-mcp-config")]
    pub strict_mcp_config: bool,
    /// `--permission-mode`.
    #[serde(rename = "--permission-mode")]
    pub permission_mode: String,
    /// `--tools`.
    #[serde(rename = "--tools")]
    pub tools: String,
}

/// `Partial<FlagArguments>`, its members in the order they were given.
pub type GivenFlags = JsObject;

/// A file the host read for a flag: whether it is a regular file, its size and its text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileFact {
    /// Whether it is a regular file.
    pub regular: bool,
    /// Its size in bytes.
    pub bytes: f64,
    /// Its text.
    pub text: String,
}

/// What an `--add-dir` path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PathKind {
    /// A directory.
    Directory,
    /// A file.
    File,
}

/// The platform the flags are read on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlagPlatform {
    /// A POSIX system.
    Posix,
    /// Windows.
    Windows,
}

/// What the host knows about the machine and the run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's record of the flags: independent yes-or-no facts, as the interface has them"
)]
pub struct FlagFacts {
    /// `--print` (`-p`): a non-interactive run.
    pub print: bool,
    /// `--resume` (`-r`): the run resumes a session.
    pub resume: bool,
    /// `--continue` (`-c`): the run continues the latest session.
    pub continuing: bool,
    /// The `agent` setting of the settings files, which `--agent` overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_setting: Option<String>,
    /// `--output-format`, when given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    /// The environment.
    pub env: IndexMap<String, Option<String>>,
    /// The platform.
    pub platform: FlagPlatform,
    /// Managed settings set `disableSideloadFlags`.
    pub sideload_disabled: bool,
    /// The files a flag names, by the path it gave; a path with no entry does not exist.
    pub files: IndexMap<String, FileFact>,
    /// What each `--add-dir` path is; a path with no entry does not exist.
    pub directories: IndexMap<String, PathKind>,
    /// The session runs in a self-hosted environment (a runner of the user's own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_hosted: Option<bool>,
    /// The MCP servers, by name, whose tool list the discovery cache holds from a previous connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_tool_lists: Option<Vec<String>>,
}

/// What the strategies of the flags are given.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlagContext {
    /// What the command line gave.
    pub given: GivenFlags,
    /// What the host knows.
    pub facts: FlagFacts,
}

/// The customizations a mode can leave out (`flags/customizations.ts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Customization {
    /// `hooks`.
    Hooks,
    /// `skills`.
    Skills,
    /// `commands`.
    Commands,
    /// `agents`.
    Agents,
    /// `installed-plugins`.
    InstalledPlugins,
    /// `mcp-servers`.
    McpServers,
    /// `auto-memory`.
    AutoMemory,
    /// `claude-md`.
    ClaudeMd,
    /// `output-styles`.
    OutputStyles,
    /// `workflows`.
    Workflows,
    /// `themes`.
    Themes,
    /// `keybindings`.
    Keybindings,
    /// `status-line`.
    StatusLine,
    /// `file-suggestion`.
    FileSuggestion,
    /// `lsp-servers`.
    LspServers,
}

/// The customizations, in the order of the table.
pub const CUSTOMIZATIONS: [Customization; 15] = [
    Customization::Hooks,
    Customization::Skills,
    Customization::Commands,
    Customization::Agents,
    Customization::InstalledPlugins,
    Customization::McpServers,
    Customization::AutoMemory,
    Customization::ClaudeMd,
    Customization::OutputStyles,
    Customization::Workflows,
    Customization::Themes,
    Customization::Keybindings,
    Customization::StatusLine,
    Customization::FileSuggestion,
    Customization::LspServers,
];

/// Where a session plugin comes from: `directory`: a folder that is a plugin or holds plugins;
/// `archive`: a `.zip` on disk; `url`: a `.zip` to fetch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionPluginSource {
    /// A folder.
    Directory,
    /// A `.zip` on disk.
    Archive,
    /// A `.zip` to fetch.
    Url,
}

/// The reserved marketplace name the plugin's id ends with: `<name>@inline`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionPluginOrigin {
    /// `inline`.
    Inline,
}

/// A plugin for this session only: `--plugin-dir` and `--plugin-url` load plugins of the `inline` origin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPluginRequest {
    /// Where it comes from.
    pub source: SessionPluginSource,
    /// The folder, archive or URL.
    pub location: String,
    /// The reserved marketplace name the plugin's id ends with: `<name>@inline`.
    pub origin: SessionPluginOrigin,
}

/// One entry of `--channels`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionChannel {
    /// The entry as typed.
    pub raw: String,
    /// What the channel verdict reads (`channelVerdict`): a `ChannelEntry`.
    pub entry: JsValue,
    /// A `server:<name>` entry names an MCP server, not a plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
}

/// Where a `--mcp-config` value comes from: a JSON string typed on the command line or a file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum McpConfigSource {
    /// A JSON string typed on the command line.
    Inline {
        /// The JSON text.
        text: String,
    },
    /// A file.
    File {
        /// Its path.
        path: String,
    },
}

/// Whether `--restricted` lets cloud sessions be created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudSessions {
    /// Allowed.
    Allowed,
    /// Refused.
    Refused,
}

/// What the flags add up to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's record of the flags: independent yes-or-no facts, as the interface has them"
)]
pub struct SessionPlan {
    /// Why startup exits with `1`: each a line on stderr. Empty when the session starts.
    pub refusals: Vec<String>,
    /// The `warnings` member of the plan.
    pub warnings: Vec<PluginIssue>,
    /// The `sessionPlugins` member of the plan.
    pub session_plugins: Vec<SessionPluginRequest>,
    /// The `skipped` member of the plan.
    pub skipped: Vec<Customization>,
    /// Whether the session reads stored login credentials (OAuth tokens, the system keychain).
    pub reads_stored_credentials: bool,
    /// Environment variables the flag sets for the session (`--bare` sets `CLAUDE_CODE_SIMPLE`).
    pub env: IndexMap<String, String>,
    /// The settings scopes that load (`PolicyScope`); absent means every one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_scopes: Option<Vec<String>>,
    /// The `--settings` layer (`PolicyLayer`), above the files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag_settings: Option<JsValue>,
    /// The `additionalDirectories` member of the plan.
    pub additional_directories: Vec<String>,
    /// The `agent` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// The `--agents` definitions, parsed: one object keyed by agent name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_agents: Option<JsObject>,
    /// The `subagentPromptAppend` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_prompt_append: Option<String>,
    /// The `forwardSubagentText` member of the plan.
    pub forward_subagent_text: bool,
    /// The `channels` member of the plan.
    pub channels: Vec<SessionChannel>,
    /// The `mcpConfigs` member of the plan.
    pub mcp_configs: Vec<McpConfigSource>,
    /// The `strictMcpConfig` member of the plan.
    pub strict_mcp_config: bool,
    /// `-p` with `--mcp-config` waits for pending servers up to this many milliseconds before the first turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_wait_ms: Option<f64>,
    /// The `--mcp-config` servers, by name, the first turn does not wait for: they connect on first use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_wait_skips: Option<Vec<String>>,
    /// The `PermissionMode` of `--permission-mode`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// The `--tools` list: `[]` disables every built-in tool, `['default']` is the default set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// `--tools` names a task-tracking tool: the session is opted in.
    pub opts_into_task_tools: bool,
    /// The `restricted` member of the plan.
    pub restricted: bool,
    /// `--restricted` confines the built-in file tools to the working directories.
    pub file_tools_confined: bool,
    /// `--restricted` refuses to create cloud sessions.
    pub cloud_sessions: CloudSessions,
    /// What `--add-dir` directories load beyond file access (`flags/add-dir.ts`).
    pub add_dir_loads: Vec<String>,
}

/// What one flag adds to the plan: lists are appended, a scalar replaces.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contribution {
    /// Why startup exits with `1`: each a line on stderr. Empty when the session starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusals: Option<Vec<String>>,
    /// The `warnings` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<PluginIssue>>,
    /// The `sessionPlugins` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_plugins: Option<Vec<SessionPluginRequest>>,
    /// The `skipped` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<Vec<Customization>>,
    /// Whether the session reads stored login credentials (OAuth tokens, the system keychain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reads_stored_credentials: Option<bool>,
    /// Environment variables the flag sets for the session (`--bare` sets `CLAUDE_CODE_SIMPLE`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<IndexMap<String, String>>,
    /// The settings scopes that load (`PolicyScope`); absent means every one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_scopes: Option<Vec<String>>,
    /// The `--settings` layer (`PolicyLayer`), above the files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag_settings: Option<JsValue>,
    /// The `additionalDirectories` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_directories: Option<Vec<String>>,
    /// The `agent` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// The `--agents` definitions, parsed: one object keyed by agent name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_agents: Option<JsObject>,
    /// The `subagentPromptAppend` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_prompt_append: Option<String>,
    /// The `forwardSubagentText` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward_subagent_text: Option<bool>,
    /// The `channels` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<SessionChannel>>,
    /// The `mcpConfigs` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_configs: Option<Vec<McpConfigSource>>,
    /// The `strictMcpConfig` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict_mcp_config: Option<bool>,
    /// `-p` with `--mcp-config` waits for pending servers up to this many milliseconds before the first turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_wait_ms: Option<f64>,
    /// The `--mcp-config` servers, by name, the first turn does not wait for: they connect on first use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_wait_skips: Option<Vec<String>>,
    /// The `PermissionMode` of `--permission-mode`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// The `--tools` list: `[]` disables every built-in tool, `['default']` is the default set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// `--tools` names a task-tracking tool: the session is opted in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opts_into_task_tools: Option<bool>,
    /// The `restricted` member of the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restricted: Option<bool>,
    /// `--restricted` confines the built-in file tools to the working directories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_tools_confined: Option<bool>,
    /// `--restricted` refuses to create cloud sessions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_sessions: Option<CloudSessions>,
    /// What `--add-dir` directories load beyond file access (`flags/add-dir.ts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add_dir_loads: Option<Vec<String>>,
}

/// A contribution that refuses the start for these reasons.
#[must_use]
pub fn refuse(reasons: Vec<String>) -> Contribution {
    Contribution {
        refusals: Some(reasons),
        ..Contribution::default()
    }
}

crate::registry::exports! {
    "cli/flags/types" {
        "FLAG_NAMES" => ok || FLAG_NAMES;
        "CUSTOMIZATIONS" => ok || CUSTOMIZATIONS;
        "refuse" => ok |reasons: Rest<String>| refuse(reasons.0);
    }
}
