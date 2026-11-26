//! Port of `src/agents/teammate.ts`, switched.
//!
//! A teammate spawned from an agent definition (agent-teams.md, "Use subagent definitions for
//! teammates"; spec §5.12.6): which parts of the definition apply, by display mode, and whether a
//! stopped teammate brought back gets its definition again. Pure: spawning, panes and the prompt are
//! the host's.
//!
//! `AgentDefinition` is the SDK's type of `protocol/official.ts`, which is not in the core yet: it
//! is the members this module reads, and what it only carries along (`effort`, an MCP server spec)
//! is a `JsValue`.

use serde::{Deserialize, Serialize};

use crate::format::agent_record::AgentSource;
use crate::js::JsValue;

/// In process (inside the lead's session, the default), or in its own split pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TeammateDisplay {
    /// Inside the lead's session.
    InProcess,
    /// In its own split pane.
    SplitPane,
}

/// The tools every in-process teammate gets beside its list: `SendMessage`, then the Task tools.
const TEAM_TOOL: &str = "SendMessage";
const TASK_TOOLS: [&str; 4] = ["TaskCreate", "TaskGet", "TaskList", "TaskUpdate"];

/// The members of an agent definition this module reads.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    /// The tools the agent may use.
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    /// The tools removed from its tools.
    #[serde(default)]
    pub disallowed_tools: Option<Vec<String>>,
    /// Its model.
    #[serde(default)]
    pub model: Option<String>,
    /// Its effort level.
    #[serde(default)]
    pub effort: Option<JsValue>,
    /// The window after which its context compacts.
    #[serde(default)]
    pub auto_compact_window: Option<f64>,
    /// Its system prompt.
    pub prompt: String,
    /// The MCP servers it is given.
    #[serde(default)]
    pub mcp_servers: Option<Vec<JsValue>>,
}

/// What a teammate is spawned from.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeammateInput {
    /// The definition the teammate is spawned from (`AgentRecord.definition`).
    pub definition: AgentDefinition,
    /// Where the teammate runs.
    pub display: TeammateDisplay,
    /// The model the spawn request names, which wins over the definition's.
    #[serde(default)]
    pub requested_model: Option<String>,
    /// The session has the Task tools.
    pub task_tools: bool,
    /// Where the definition came from: a plugin agent's `mcpServers` stays ignored (§5.6.1).
    pub source: AgentSource,
}

/// Where the body goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptPlacement {
    /// Appended to the default system prompt.
    Append,
    /// In place of it.
    Replace,
}

/// The body and where it goes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TeammatePrompt {
    /// Where the body goes.
    pub placement: PromptPlacement,
    /// The body.
    pub text: String,
}

/// What the definition gives the teammate; a field left out is the teammate's default.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeammateSetup {
    /// The tools from the definition's `tools`, with the team tools in process; absent without a list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Removed from the teammate's tools (in process only), never the team tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disallowed_tools: Option<Vec<String>>,
    /// The effort level of the definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<JsValue>,
    /// The compaction window of the definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_window: Option<f64>,
    /// Applied in a split pane only, and never a plugin agent's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<JsValue>>,
    /// The body: appended to the default system prompt in process, in place of it in a split pane.
    pub prompt: TeammatePrompt,
    /// The model: the request's, else the definition's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

fn team_tools(task_tools: bool) -> Vec<&'static str> {
    if task_tools {
        let mut tools = vec![TEAM_TOOL];
        tools.extend(TASK_TOOLS);
        tools
    } else {
        vec![TEAM_TOOL]
    }
}

/// A plugin agent's `mcpServers` is never applied (§5.6.1).
const fn plugin_mcp_ignored(source: AgentSource) -> bool {
    match source {
        AgentSource::Managed | AgentSource::Flag | AgentSource::Project | AgentSource::User => {
            false
        }
        AgentSource::Plugin => true,
    }
}

/// The first of each distinct tool, in order: `[...new Set(tools)]`.
fn distinct(tools: Vec<String>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for tool in tools {
        if !kept.contains(&tool) {
            kept.push(tool);
        }
    }
    kept
}

fn in_process(input: &TeammateInput) -> TeammateSetup {
    let TeammateInput {
        definition,
        task_tools,
        ..
    } = input;
    let team = team_tools(*task_tools);
    let tools = definition.tools.as_ref().map(|listed| {
        let mut all = listed.clone();
        all.extend(team.iter().map(|tool| (*tool).to_owned()));
        all
    });
    let removed = definition.disallowed_tools.as_ref().map(|listed| {
        listed
            .iter()
            .filter(|tool| !team.contains(&tool.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    });
    TeammateSetup {
        tools: tools.map(distinct),
        disallowed_tools: removed,
        effort: definition.effort.clone(),
        auto_compact_window: definition.auto_compact_window,
        mcp_servers: None,
        prompt: TeammatePrompt {
            placement: PromptPlacement::Append,
            text: definition.prompt.clone(),
        },
        model: None,
    }
}

fn split_pane(input: &TeammateInput) -> TeammateSetup {
    let TeammateInput {
        definition, source, ..
    } = input;
    let servers = if plugin_mcp_ignored(*source) {
        None
    } else {
        definition.mcp_servers.clone()
    };
    TeammateSetup {
        tools: definition.tools.clone(),
        disallowed_tools: None,
        effort: None,
        auto_compact_window: None,
        mcp_servers: servers,
        prompt: TeammatePrompt {
            placement: PromptPlacement::Replace,
            text: definition.prompt.clone(),
        },
        model: None,
    }
}

/// What each display applies beyond `model` (agent-teams.md's table).
fn by_display(input: &TeammateInput) -> TeammateSetup {
    match input.display {
        TeammateDisplay::InProcess => in_process(input),
        TeammateDisplay::SplitPane => split_pane(input),
    }
}

/// The parts of a definition a teammate gets. `skills` is never applied: a teammate loads skills
/// (and, in process, MCP servers) from the project and user settings.
#[must_use]
pub fn teammate_setup(input: &TeammateInput) -> TeammateSetup {
    let model = input
        .requested_model
        .clone()
        .or_else(|| input.definition.model.clone());
    TeammateSetup {
        model,
        ..by_display(input)
    }
}

/// The notice after the resume report when a stopped teammate comes back without its definition.
pub const DEFINITION_NOT_RESTORED: &str = "Its agent definition was not restored: the folder its definition file came from is not trusted (source: projectSettings), so the teammate is running with the team-essential tools and no custom instructions. To restore it, the user needs to run Claude Code in that folder once and accept the trust dialog (the --debug log names the folder); do not change trust settings on the user's behalf.";

/// Whose definition is re-applied only from a trusted folder: the `.claude/agents/` of the project or an `--add-dir` directory.
const fn needs_trusted_folder(source: AgentSource) -> bool {
    match source {
        AgentSource::Managed | AgentSource::Flag | AgentSource::User | AgentSource::Plugin => false,
        AgentSource::Project => true,
    }
}

/// What a stopped teammate is brought back from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeammateRestore {
    /// The definition's source (`project` also stands for an `--add-dir` directory).
    pub source: AgentSource,
    /// The folder holding the agent file is itself trusted; a trusted parent does not count.
    pub folder_trusted: bool,
}

/// Whether the definition came back, and the notice when it did not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Restored {
    /// The definition is applied again.
    pub reapplied: bool,
    /// Follows the resume report in the sender's tool result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<&'static str>,
}

/// Bringing back a stopped in-process teammate: its definition is re-applied, or it returns with
/// only the team tools and `notice` follows the resume report in the sender's tool result.
#[must_use]
pub const fn restore_teammate(input: TeammateRestore) -> Restored {
    let reapplied = !needs_trusted_folder(input.source) || input.folder_trusted;
    Restored {
        reapplied,
        notice: if reapplied {
            None
        } else {
            Some(DEFINITION_NOT_RESTORED)
        },
    }
}

crate::registry::exports! {
    "agents/teammate" {
        "teammateSetup" => ok |input: TeammateInput| teammate_setup(&input);
        "DEFINITION_NOT_RESTORED" => ok || DEFINITION_NOT_RESTORED;
        "restoreTeammate" => ok |input: TeammateRestore| restore_teammate(input);
    }
}
