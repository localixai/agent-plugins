//! Port of `src/agents/tool-list.ts`, switched.
//!
//! One entry of the `tools` or `disallowedTools` field (sub-agents.md, "Restrict which tools a
//! subagent can use"): a tool name, with the specifier it was written with, or `Agent(type, ...)`.

use serde::{Deserialize, Serialize};

use crate::js::trim;
use crate::rules::target::mcp::mcp_tool_rule_matches;
use crate::rules::tool_pattern::{is_anchored_allow_glob, wildcard_reg_exp};
use crate::rules::types::PermissionBehavior;

/// Only for `Agent`: the subagent types it may spawn, or `any`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Spawn {
    /// `Agent` alone allows any.
    Any(AnySpawn),
    /// `Agent(worker, researcher)` is an allowlist.
    Types(Vec<String>),
}

/// The text `any`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnySpawn {
    /// `any`.
    #[serde(rename = "any")]
    Any,
}

/// One entry of the `tools` or `disallowedTools` field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEntry {
    /// The tool name; `Task` (the pre-2.1.63 name, "still work as aliases") reads as `Agent`.
    pub name: String,
    /// The text in parentheses (`git push *` of `Bash(git push *)`); not for `Agent`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specifier: Option<String>,
    /// Only for `Agent`: the subagent types it may spawn, or `any`. "Restrict which subagents can be
    /// spawned": `Agent(worker, researcher)` is an allowlist, `Agent` alone allows any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn: Option<Spawn>,
}

/// The Agent tool.
pub const AGENT_TOOL: &str = "Agent";
const AGENT_NAMES: [&str; 2] = [AGENT_TOOL, "Task"];

/// Whether a tool name is the Agent tool: `Task` (its name before 2.1.63) "still work\[s\] as an alias".
#[must_use]
pub fn is_agent_tool_name(name: &str) -> bool {
    AGENT_NAMES.contains(&name)
}

/// `Name(content)` splits at the first `(` when the entry ends in `)`; anything else is a name.
fn split_entry(text: &str) -> (String, Option<String>) {
    match text.find('(') {
        Some(open) if open > 0 && text.ends_with(')') => (
            trim(text.get(..open).unwrap_or_default()).to_owned(),
            text.get(open + 1..text.len() - 1).map(str::to_owned),
        ),
        _ => (text.to_owned(), None),
    }
}

fn type_list(content: &str) -> Vec<String> {
    content
        .split(',')
        .map(|one| trim(one).to_owned())
        .filter(|one| !one.is_empty())
        .collect()
}

/// Reads one entry of the field, as `readAgent` keeps it (a comma inside parentheses is not a split).
#[must_use]
pub fn parse_tool_entry(entry: &str) -> ToolEntry {
    let (name, content) = split_entry(trim(entry));
    if AGENT_NAMES.contains(&name.as_str()) {
        let types = content.as_deref().map(type_list).unwrap_or_default();
        return ToolEntry {
            name: AGENT_TOOL.to_owned(),
            specifier: None,
            spawn: Some(if types.is_empty() {
                Spawn::Any(AnySpawn::Any)
            } else {
                Spawn::Types(types)
            }),
        };
    }
    ToolEntry {
        name,
        specifier: content,
        spawn: None,
    }
}

/// Whether `entry` names the tool `toolName`. The behaviour is the one permissions.md gives the same
/// name in a rule: `tools` reads like an allow rule and `disallowedTools` like a deny rule
/// (spec/judgments.md). So `mcp__reports__v2` never matches in `tools` and does in `disallowedTools`,
/// and an unanchored glob such as `mcp__*` matches only as a deny entry. A specifier never narrows
/// the match: "still removes the whole tool".
#[must_use]
pub fn tool_entry_matches(
    entry: &ToolEntry,
    tool_name: &str,
    behavior: PermissionBehavior,
) -> bool {
    let name = &entry.name;
    if !name.contains('*') {
        return mcp_tool_rule_matches(name, tool_name, behavior);
    }
    let anchored = behavior != PermissionBehavior::Allow || is_anchored_allow_glob(name);
    anchored && wildcard_reg_exp(name, None).is_ok_and(|mut glob| glob.test(tool_name))
}

crate::registry::exports! {
    "agents/tool-list" {
        "AGENT_TOOL" => ok || AGENT_TOOL;
        "isAgentToolName" => ok |name: String| is_agent_tool_name(&name);
        "parseToolEntry" => ok |entry: String| parse_tool_entry(&entry);
        "toolEntryMatches" => ok |entry: ToolEntry, tool_name: String, behavior: PermissionBehavior|
            tool_entry_matches(&entry, &tool_name, behavior);
    }
}
