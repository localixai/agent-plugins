//! Port of `src/agents/catalog/types.ts`, switched.
//!
//! One subagent a session can run, whatever it came from (sub-agents.md, "Choose the subagent scope"
//! and "Built-in subagents"): a built-in, the `--agents` flag, a file of the managed, project, user or
//! added-directory folders, or a plugin. The catalog holds these in one shape so a call, an @-mention
//! and `--agent` ask one question. The tier is the row of the Priority column a source ranks by.

use serde::{Deserialize, Serialize};

use crate::format::agent_record::{AgentExtras, AgentRecord, AgentSource, FileSource};
use crate::js::JsValue;

/// The row of the Priority column a source ranks by: an agent source, or the built-ins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CatalogTier {
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
    /// One of Claude Code's own.
    Builtin,
}

/// The scope folders an agent file is read from; `project` also stands for an added directory.
pub type AgentFolderScope = FileSource;

/// Where a catalog agent comes from, in the shape `SessionSkill.source` has (`format/component-source`):
/// a plugin by the id the host uses for it, a scope folder, the `--agents` flag (or the SDK `agents`
/// option), or Claude Code's built-ins. `catalog_tier` gives the row of the Priority column it ranks by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum CatalogSource {
    /// A component a plugin brought: the plugin's id as the host names it (`name@marketplace`, or the bare name).
    Plugin {
        /// The plugin's id.
        plugin_id: String,
    },
    /// A component read from a scope folder.
    Folder {
        /// The scope.
        scope: AgentFolderScope,
    },
    /// The `--agents` flag.
    Flag,
    /// Claude Code's built-ins.
    Builtin,
}

/// The row of the Priority column (sub-agents.md, "Choose the subagent scope") a source ranks by.
#[must_use]
pub const fn catalog_tier(source: &CatalogSource) -> CatalogTier {
    match source {
        CatalogSource::Folder { scope } => match scope {
            FileSource::Managed => CatalogTier::Managed,
            FileSource::Project => CatalogTier::Project,
            FileSource::User => CatalogTier::User,
        },
        CatalogSource::Plugin { .. } => CatalogTier::Plugin,
        CatalogSource::Flag => CatalogTier::Flag,
        CatalogSource::Builtin => CatalogTier::Builtin,
    }
}

/// What a subagent loads when it starts (sub-agents.md, "Built-in subagents" and "What loads at startup").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartContext {
    /// The user, project and local CLAUDE.md files.
    pub claude_md: bool,
    /// The git status snapshot.
    pub git_status: bool,
}

/// One subagent of the catalog.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogAgent {
    /// The identifier a call, an @-mention, `--agent` and a hook matcher use: the frontmatter name, or for
    /// a plugin agent its scoped name (`my-plugin:review:security`).
    pub name: String,
    /// The name without the plugin and folder scope: what `--agent security-reviewer` finds.
    pub own_name: String,
    /// Where it comes from.
    pub source: CatalogSource,
    /// The official shape (`AgentDefinition`); a built-in has an empty prompt (its own is Claude Code's).
    pub definition: JsValue,
    /// What the official shape has no key for.
    pub extras: AgentExtras,
    /// The frontmatter `hooks`; never set for a plugin or built-in agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<JsValue>,
    /// The file, or `--agents.<name>`; absent for a built-in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// What it loads when it starts.
    pub starts_with: StartContext,
}

/// The source of a record; a plugin record names the plugin by `plugin_id`.
fn source_of(source: AgentSource, plugin_id: &str) -> CatalogSource {
    match source {
        AgentSource::Managed => CatalogSource::Folder {
            scope: FileSource::Managed,
        },
        AgentSource::Flag => CatalogSource::Flag,
        AgentSource::Project => CatalogSource::Folder {
            scope: FileSource::Project,
        },
        AgentSource::User => CatalogSource::Folder {
            scope: FileSource::User,
        },
        AgentSource::Plugin => CatalogSource::Plugin {
            plugin_id: plugin_id.to_owned(),
        },
    }
}

/// A plugin record's plugin name: its scoped name up to the first `:`.
fn plugin_name_of(record: &AgentRecord) -> &str {
    record.scoped_name.split(':').next().unwrap_or_default()
}

/// The catalog entry of an agent read from a file, the flag or a plugin. `plugin_id` is the id the host
/// uses for the plugin of a plugin record (a session passes it); without it the plugin's name stands.
#[must_use]
pub fn catalog_agent_of(record: &AgentRecord, plugin_id: Option<&str>) -> CatalogAgent {
    CatalogAgent {
        name: record.scoped_name.clone(),
        own_name: record.name.clone(),
        source: source_of(
            record.source,
            plugin_id.unwrap_or_else(|| plugin_name_of(record)),
        ),
        definition: record.definition.clone(),
        extras: record.extras.clone(),
        hooks: record.hooks.clone(),
        path: Some(record.path.clone()),
        // "unless you set `omitClaudeMd` in a custom subagent's definition to start it without the user,
        // project, and local CLAUDE.md files"; the git status snapshot is the one every custom agent loads.
        starts_with: StartContext {
            claude_md: record
                .definition
                .get("omitClaudeMd")
                .and_then(JsValue::as_bool)
                != Some(true),
            git_status: true,
        },
    }
}

/// Which tiers count as built-in; a registry over the tiers, so no code compares one to a string.
const fn builtin_source(tier: CatalogTier) -> bool {
    match tier {
        CatalogTier::Builtin => true,
        CatalogTier::Managed
        | CatalogTier::Flag
        | CatalogTier::Project
        | CatalogTier::User
        | CatalogTier::Plugin => false,
    }
}

/// Whether the agent is one of Claude Code's own.
#[must_use]
pub const fn is_builtin_agent(agent: &CatalogAgent) -> bool {
    builtin_source(catalog_tier(&agent.source))
}

crate::registry::exports! {
    "agents/catalog/types" {
        "catalogTier" => ok |source: CatalogSource| catalog_tier(&source);
        "catalogAgentOf" => ok |record: AgentRecord, plugin_id: Option<String>| catalog_agent_of(&record, plugin_id.as_deref());
        "isBuiltinAgent" => ok |agent: CatalogAgent| is_builtin_agent(&agent);
    }
}
