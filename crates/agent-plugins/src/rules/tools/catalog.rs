//! Port of `src/rules/tools/catalog.ts`, switched.

use serde::Serialize;

use super::rule_formats::RuleFormatId;
use crate::protocol::tool_names::{DOCUMENTED_TOOLS, REMOVED_TOOLS, SDK_ONLY_TOOLS};
use crate::registry::Set;
use indexmap::IndexMap;

/// What tools-reference.md says about a tool in rules: the "Permission required" column of the tools
/// table and the "Rule format" table ("Configure tools with permission rules and hooks"), whose
/// "Applies to" column names the tools each format covers; "Tools not listed here ... accept only
/// the bare tool name with no specifier".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogToolEntry {
    /// The tool prompts in Manual mode for paths inside the working directory.
    pub permission_required: bool,
    /// How the tool reads what follows its name inside the parentheses of a rule.
    pub rule_format: RuleFormatId,
}

const fn yes(rule_format: RuleFormatId) -> CatalogToolEntry {
    CatalogToolEntry {
        permission_required: true,
        rule_format,
    }
}
const fn no(rule_format: RuleFormatId) -> CatalogToolEntry {
    CatalogToolEntry {
        permission_required: false,
        rule_format,
    }
}

/// One entry per documented tool.
const TOOL_CATALOG: [(&str, CatalogToolEntry); 46] = [
    ("Agent", no(RuleFormatId::SubagentType)),
    ("Artifact", yes(RuleFormatId::Bare)),
    ("AskUserQuestion", no(RuleFormatId::Bare)),
    ("Bash", yes(RuleFormatId::Command)),
    ("CronCreate", no(RuleFormatId::Bare)),
    ("CronDelete", no(RuleFormatId::Bare)),
    ("CronList", no(RuleFormatId::Bare)),
    ("Edit", yes(RuleFormatId::EditPath)),
    ("EndConversation", no(RuleFormatId::Bare)),
    ("EnterPlanMode", no(RuleFormatId::Bare)),
    ("EnterWorktree", yes(RuleFormatId::Bare)),
    ("ExitPlanMode", yes(RuleFormatId::Bare)),
    ("ExitWorktree", no(RuleFormatId::Bare)),
    ("Glob", no(RuleFormatId::ReadPath)),
    ("Grep", no(RuleFormatId::ReadPath)),
    ("ListAgents", no(RuleFormatId::Bare)),
    ("ListMcpResourcesTool", no(RuleFormatId::Bare)),
    ("LSP", no(RuleFormatId::ReadPath)),
    ("Monitor", yes(RuleFormatId::Command)),
    ("NotebookEdit", yes(RuleFormatId::EditPath)),
    ("PowerShell", yes(RuleFormatId::PowershellCommand)),
    ("PushNotification", no(RuleFormatId::Bare)),
    ("Read", no(RuleFormatId::ReadPath)),
    ("ReadMcpResourceTool", no(RuleFormatId::Bare)),
    ("RemoteTrigger", no(RuleFormatId::Bare)),
    ("ReportFindings", no(RuleFormatId::Bare)),
    ("ScheduleWakeup", no(RuleFormatId::Bare)),
    ("SendFeedback", no(RuleFormatId::Bare)),
    ("SendMessage", no(RuleFormatId::Bare)),
    ("SendUserFile", no(RuleFormatId::Bare)),
    ("ShareOnboardingGuide", yes(RuleFormatId::Bare)),
    ("Skill", yes(RuleFormatId::SkillName)),
    ("SubagentHandback", no(RuleFormatId::Bare)),
    ("TaskCreate", no(RuleFormatId::Bare)),
    ("TaskGet", no(RuleFormatId::Bare)),
    ("TaskList", no(RuleFormatId::Bare)),
    ("TaskOutput", no(RuleFormatId::Bare)),
    ("TaskStop", no(RuleFormatId::Bare)),
    ("TaskUpdate", no(RuleFormatId::Bare)),
    ("TodoWrite", no(RuleFormatId::Bare)),
    ("ToolSearch", no(RuleFormatId::Bare)),
    ("WaitForMcpServers", no(RuleFormatId::Bare)),
    ("WebFetch", yes(RuleFormatId::Domain)),
    ("WebSearch", yes(RuleFormatId::Bare)),
    ("Workflow", yes(RuleFormatId::Bare)),
    ("Write", yes(RuleFormatId::EditPath)),
];

/// `TOOL_CATALOG`, by tool.
fn tool_catalog() -> IndexMap<&'static str, CatalogToolEntry> {
    TOOL_CATALOG.into_iter().collect()
}

/// The entry of a documented tool, or `None` for any other name (an MCP tool, a typo).
#[must_use]
pub fn catalog_entry(tool_name: &str) -> Option<CatalogToolEntry> {
    TOOL_CATALOG
        .iter()
        .find(|(name, _)| *name == tool_name)
        .map(|(_, entry)| *entry)
}

fn is_removed(tool: &str) -> bool {
    REMOVED_TOOLS.contains(&tool)
}

/// The tool names a rule can name without a typo warning: the documented tools Claude Code still
/// has and the SDK's own tools the docs leave undescribed. Pass it to `ruleIssues`.
#[must_use]
pub fn known_tools() -> Set<&'static str> {
    Set {
        members: DOCUMENTED_TOOLS
            .iter()
            .filter(|tool| !is_removed(tool))
            .chain(SDK_ONLY_TOOLS)
            .copied()
            .collect(),
    }
}

/// Whether `tool_name` is a built-in tool Claude Code has (the removed `TaskOutput` is not).
#[must_use]
pub fn is_known_tool(tool_name: &str) -> bool {
    (DOCUMENTED_TOOLS.contains(&tool_name) && !is_removed(tool_name))
        || SDK_ONLY_TOOLS.contains(&tool_name)
}

/// `DecideContext.requiresPermission` for the kit's catalog: the "Permission required" column for a
/// documented tool, and `true` for any other name (an MCP tool prompts "on first use of each tool",
/// mcp.md; spec/judgments.md).
#[must_use]
pub fn tool_requires_permission(tool_name: &str) -> bool {
    catalog_entry(tool_name).is_none_or(|entry| entry.permission_required)
}

crate::registry::exports! {
    "rules/tools/catalog" {
        "TOOL_CATALOG" => ok tool_catalog;
        "catalogEntry" => ok |tool_name: String| catalog_entry(&tool_name);
        "KNOWN_TOOLS" => ok known_tools;
        "isKnownTool" => ok |tool_name: String| is_known_tool(&tool_name);
        "toolRequiresPermission" => ok |tool_name: String| tool_requires_permission(&tool_name);
    }
}
