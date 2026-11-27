//! Port of `src/format/agent-name.ts`, switched.
//!
//! What makes an agent's name usable and how a plugin agent is named (sub-agents.md, "Frontmatter
//! reference", "Subagent files Claude Code skips"; plugins/components.md, "Organize agents in
//! subfolders").

use serde::{Deserialize, Serialize};

use super::agent_record::AgentSource;
use crate::js::code_unit_length;

/// sub-agents.md, "Frontmatter reference": a name is "at most 256 characters".
pub const MAX_AGENT_NAME: usize = 256;

/// Why a name cannot be used by a project, user, managed or `--add-dir` agent, or `None` when it
/// can (sub-agents.md, "Subagent files Claude Code skips": a name that "starts with `-`, contains
/// `:`, or is longer than 256 characters"). Judgment: length counts UTF-16 code units (spec/judgments.md).
#[must_use]
pub fn agent_name_problem(name: &str) -> Option<String> {
    if code_unit_length(name) > MAX_AGENT_NAME {
        return Some(format!("is longer than {MAX_AGENT_NAME} characters"));
    }
    if name.starts_with('-') {
        return Some("starts with \"-\"".to_owned());
    }
    if name.contains(':') {
        return Some("contains \":\", which is reserved for plugin-scoped identifiers such as my-plugin:reviewer".to_owned());
    }
    None
}

/// Whether a name is usable: not empty and with no problem.
#[must_use]
pub fn valid_agent_name(name: &str) -> bool {
    !name.is_empty() && agent_name_problem(name).is_none()
}

/// `/[\\/]/`.
const fn is_separator(character: char) -> bool {
    matches!(character, '\\' | '/')
}

/// The last segment of a path without its `.md`: what a plugin agent is named without a `name`.
/// Judgment (spec/judgments.md): a segment that is nothing but `.md` keeps its whole text, so a
/// name is never empty.
#[must_use]
pub fn agent_file_name(path: &str) -> String {
    let last = path.rsplit(is_separator).next().unwrap_or(path);
    // `/\.md$/i`: the case folds ASCII letters only.
    let stem = last
        .len()
        .checked_sub(3)
        .filter(|at| {
            last.as_bytes()
                .get(*at..)
                .is_some_and(|tail| tail.eq_ignore_ascii_case(b".md"))
        })
        .and_then(|at| last.get(..at))
        .unwrap_or(last);
    if stem.is_empty() { last } else { stem }.to_owned()
}

/// The name a plugin agent is registered under (plugins/components.md, "Organize agents in
/// subfolders"): the plugin name, each subfolder of `relativePath` and `name`, joined by colons.
/// `name` takes the place of the file name only, so `review/security.md` with `name: audit` is
/// `my-plugin:review:audit`. A file the manifest lists has no `relativePath`: no subfolder names.
#[must_use]
pub fn scoped_agent_name(plugin: &str, relative_path: Option<&str>, name: &str) -> String {
    let segments: Vec<&str> = relative_path
        .map(|path| path.split(is_separator).collect())
        .unwrap_or_default();
    let folders = segments
        .split_last()
        .map(|(_, folders)| folders)
        .unwrap_or_default()
        .iter()
        .filter(|segment| !segment.is_empty() && **segment != ".");
    std::iter::once(plugin)
        .chain(folders.copied())
        .chain(std::iter::once(name))
        .collect::<Vec<_>>()
        .join(":")
}

/// What `hookAgentType` reads of an agent: `Pick<AgentRecord, 'name' | 'scopedName' | 'source'>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookAgent {
    /// The agent's name.
    pub name: String,
    /// The plugin-scoped name.
    pub scoped_name: String,
    /// Where the agent comes from.
    pub source: AgentSource,
}

/// The `agent_type` hooks receive for this agent, and the value a hook matcher is matched against.
/// hooks.md, "SubagentStart": a plugin agent reports `my-plugin:reviewer`, not the bare name.
#[must_use]
pub fn hook_agent_type(agent: &HookAgent) -> String {
    match agent.source {
        AgentSource::Managed | AgentSource::Flag | AgentSource::Project | AgentSource::User => {
            agent.name.clone()
        }
        AgentSource::Plugin => agent.scoped_name.clone(),
    }
}

crate::registry::exports! {
    "format/agent-name" {
        "MAX_AGENT_NAME" => ok || MAX_AGENT_NAME;
        "agentNameProblem" => ok |name: String| agent_name_problem(&name);
        "validAgentName" => ok |name: String| valid_agent_name(&name);
        "agentFileName" => ok |path: String| agent_file_name(&path);
        "scopedAgentName" => ok |plugin: String, relative_path: Option<String>, name: String| scoped_agent_name(&plugin, relative_path.as_deref(), &name);
        "hookAgentType" => ok |agent: HookAgent| hook_agent_type(&agent);
    }
}
