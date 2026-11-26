//! Ported from `src/rules/tools/names.ts`; only the core uses it now, so it has no wrapper.

use crate::js::trim;

/// The tool name an entry of a tool list or flag spells: `Bash(git push *)` names `Bash`.
#[must_use]
pub fn tool_name_of_entry(entry: &str) -> &str {
    let name = entry.split_once('(').map_or(entry, |(name, _)| name);
    trim(name)
}

const MCP_PREFIX: &str = "mcp__";

/// An MCP tool: `mcp__<server>__<tool>`.
#[must_use]
pub fn is_mcp_tool_name(tool_name: &str) -> bool {
    tool_name.starts_with(MCP_PREFIX)
}
