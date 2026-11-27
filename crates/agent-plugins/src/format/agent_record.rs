//! Port of `src/format/agent-record.ts`, switched.
//!
//! The shapes of one agent file as Claude Code would load it. The types of modules not in the
//! core yet (`AgentDefinition`, `HooksSettings`, `AgentColor`, `AgentIsolation`, `CacheTtl`) are
//! `JsValue`s until their modules are.

use serde::{Deserialize, Serialize};

use super::issues::PluginIssue;
use crate::js::JsValue;

/// Where an agent file comes from, in priority order (sub-agents.md, "Choose the subagent scope":
/// managed, `--agents` flag, project, user, plugin; the first that defines a name wins).
pub const AGENT_SOURCES: [AgentSource; 5] = [
    AgentSource::Managed,
    AgentSource::Flag,
    AgentSource::Project,
    AgentSource::User,
    AgentSource::Plugin,
];

/// One of `AGENT_SOURCES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentSource {
    /// An organisation's managed agent.
    Managed,
    /// An agent of the `--agents` flag.
    Flag,
    /// A project's agent.
    Project,
    /// The user's agent.
    User,
    /// A plugin's agent.
    Plugin,
}

/// What a host knows about an agent file besides its text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentOrigin {
    /// A file of a project, the user or the managed settings.
    File(FileOrigin),
    /// A file of a plugin.
    Plugin(PluginOrigin),
}

/// The source of a file origin: `Exclude<AgentSource, 'plugin' | 'flag'>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileSource {
    /// An organisation's managed agent.
    Managed,
    /// A project's agent.
    Project,
    /// The user's agent.
    User,
}

/// An agent file that is no plugin's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOrigin {
    /// The file as the host names it: used for issues and for the file name fallback.
    pub path: String,
    /// `project` also stands for a directory added with `--add-dir` (judgment, spec/judgments.md). The
    /// `--agents` flag is JSON, not a file, so `flag` is no source of a file read here.
    pub source: FileSource,
}

/// The one source of a flag origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlagSource {
    /// An agent of the `--agents` flag.
    Flag,
}

/// An agent of the `--agents` flag: JSON, not a file, so `path` is how the host names the definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagOrigin {
    /// How the host names the definition.
    pub path: String,
    /// Always `flag`.
    pub source: FlagSource,
}

/// The one source of a plugin origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginSource {
    /// A plugin's agent.
    Plugin,
}

/// An agent file of a plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginOrigin {
    /// The file as the host names it.
    pub path: String,
    /// Always `plugin`.
    pub source: PluginSource,
    /// The plugin the file belongs to.
    pub plugin: PluginOfOrigin,
}

/// The plugin of a [`PluginOrigin`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginOfOrigin {
    /// The plugin's name.
    pub name: String,
    /// The path below the plugin's `agents/` folder, such as `review/security.md`; undefined for a
    /// file the manifest's `agents` field lists, which loads without subfolder names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
}

/// Fields the official `AgentDefinition` has no key for (sub-agents.md, "Frontmatter reference").
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExtras {
    /// An `AgentColor`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<JsValue>,
    /// An `AgentIsolation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation: Option<JsValue>,
    /// `experimental.cacheTtl`, a `CacheTtl`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<JsValue>,
}

/// A top-level field Claude Code does not recognize.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnrecognizedField {
    /// The field's name.
    pub field: String,
    /// A supported field name that is close to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// One agent file as Claude Code would load it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecord {
    /// The frontmatter name, or the file name when it is missing or unusable (plugin agents only).
    pub name: String,
    /// `my-plugin:review:security` for a plugin agent; the name for every other source.
    pub scoped_name: String,
    /// Where the file comes from.
    pub source: AgentSource,
    /// The file's path.
    pub path: String,
    /// The official shape (`AgentDefinition`); `prompt` is the Markdown body.
    pub definition: JsValue,
    /// What the official shape has no key for.
    pub extras: AgentExtras,
    /// The frontmatter `hooks`, in the settings' shape (`HooksSettings`); never set for a plugin agent.
    /// Absent, it is the package's `hooks: undefined`, a member its readers test for.
    #[serde(default, serialize_with = "crate::js::value::serialize_or_undefined")]
    pub hooks: Option<JsValue>,
    /// Fields in the file that Claude Code does not apply for this source.
    pub ignored: Vec<String>,
    /// Up to five top-level fields Claude Code does not recognize.
    pub unrecognized: Vec<UnrecognizedField>,
}

/// Why a file was skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkipReason {
    /// No name, so Claude Code reads the file as a note beside the agents.
    Documentation,
    /// The file is invalid.
    Invalid,
}

/// What reading one agent file gave.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum AgentReadResult {
    /// The agent loaded.
    Loaded {
        /// The agent.
        agent: Box<AgentRecord>,
        /// What was found on the way.
        issues: Vec<PluginIssue>,
    },
    /// The file was skipped.
    Skipped {
        /// Why.
        reason: SkipReason,
        /// Always `undefined`: the package's `agent: undefined`, a member its readers test for.
        #[serde(default)]
        agent: JsValue,
        /// What was found on the way.
        issues: Vec<PluginIssue>,
    },
}

crate::registry::exports! {
    "format/agent-record" {
        "AGENT_SOURCES" => ok || AGENT_SOURCES;
    }
}
