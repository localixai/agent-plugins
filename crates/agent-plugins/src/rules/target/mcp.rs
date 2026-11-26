//! Port of `src/rules/target/mcp.ts`, switched.
//!
//! MCP rules (permissions.md, "MCP"). A tool's callable name is `mcp__<server>__<tool>`, where the
//! server is the name Claude Code gives it (mcp.md: every character outside `A-Z`, `a-z`, `0-9`,
//! `_` and `-` becomes `_`). A rule names the server (`mcp__puppeteer`, `mcp__puppeteer__*`) or one
//! tool (`mcp__puppeteer__puppeteer_navigate`).

pub use crate::format::mcp_tool::{
    mcp_tool_name, normalize_mcp_server_name, plugin_mcp_server_name,
};
use crate::rules::types::{PermissionBehavior, PermissionRule};

const PREFIX: &str = "mcp__";
const SEPARATOR: &str = "__";

/// An allow rule reads `mcp__<name>` as a server only when the name holds no `__` and does not end
/// in `_`: `mcp__reports__v2` is the tool `v2` of the server `reports`, and a trailing `_` would run
/// into the separator ("A `<server>` that contains `__` or ends in `_`: write the wildcard form").
fn is_server_form(rule_tool_name: &str) -> bool {
    let server = rule_tool_name
        .strip_prefix(PREFIX)
        .unwrap_or(rule_tool_name);
    !server.contains(SEPARATOR) && !server.ends_with('_')
}

/// Whether `toolName` is a tool of the server a bare allow rule names. The call carries no separate
/// server name, so a tool counts only when what follows the rule's name and `__` is a plain tool
/// name: `mcp__reports` does not cover `mcp__reports__v2__x`, which belongs to `reports__v2`.
fn allow_covers_server(rule_tool_name: &str, tool_name: &str) -> bool {
    let start = format!("{rule_tool_name}{SEPARATOR}");
    is_server_form(rule_tool_name)
        && tool_name
            .strip_prefix(&start)
            .is_some_and(|rest| !rest.contains(SEPARATOR))
}

/// A deny or ask rule also covers every tool whose name starts with the rule's name and `__`.
fn covers_by_prefix(rule_tool_name: &str, tool_name: &str) -> bool {
    tool_name.starts_with(&format!("{rule_tool_name}{SEPARATOR}"))
}

/// `BEYOND_THE_EXACT_NAME[behavior]`.
fn beyond_the_exact_name(
    behavior: PermissionBehavior,
    rule_tool_name: &str,
    tool_name: &str,
) -> bool {
    match behavior {
        PermissionBehavior::Allow => allow_covers_server(rule_tool_name, tool_name),
        PermissionBehavior::Deny | PermissionBehavior::Ask => {
            covers_by_prefix(rule_tool_name, tool_name)
        }
    }
}

/// Whether a rule's tool name, written without a `*`, covers the call of `toolName`. The name itself
/// always does. An `mcp__` name also covers the tools of the server it names; the three behaviours
/// differ for a server whose name holds `__` (permissions.md: as a deny or ask rule `mcp__reports`
/// and `mcp__reports__v2` also cover every tool of `reports__v2`, as an allow rule they do not).
#[must_use]
pub fn mcp_tool_rule_matches(
    rule_tool_name: &str,
    tool_name: &str,
    behavior: PermissionBehavior,
) -> bool {
    if rule_tool_name == tool_name {
        return true;
    }
    rule_tool_name.starts_with(PREFIX) && beyond_the_exact_name(behavior, rule_tool_name, tool_name)
}

/// The Cowork tools a bare `Bash` or `WebFetch` deny rule also blocks (permissions.md, "MCP"): in a
/// Cowork session Claude runs commands and fetches through these, and "Allow rules don't carry over".
const COWORK_TOOL_OF: [(&str, &str); 2] = [
    ("mcp__workspace__bash", "Bash"),
    ("mcp__workspace__web_fetch", "WebFetch"),
];

/// Only a deny rule reaches the Cowork twin of a tool: "Allow rules don't carry over". An ask rule does not (spec/judgments.md).
const fn covers_cowork_twin(behavior: PermissionBehavior) -> bool {
    match behavior {
        PermissionBehavior::Deny => true,
        PermissionBehavior::Allow | PermissionBehavior::Ask => false,
    }
}

/// Whether `rule` is a deny rule that names a whole built-in tool and `toolName` is that tool's Cowork twin.
#[must_use]
pub fn deny_covers_cowork_tool(rule: &PermissionRule, tool_name: &str) -> bool {
    covers_cowork_twin(rule.behavior)
        && rule.specifier.is_none()
        && COWORK_TOOL_OF
            .iter()
            .find(|(twin, _)| *twin == tool_name)
            .is_some_and(|(_, tool)| *tool == rule.value.tool_name)
}

crate::registry::exports! {
    "rules/target/mcp" {
        "mcpToolRuleMatches" => ok |rule_tool_name: String, tool_name: String, behavior: PermissionBehavior| mcp_tool_rule_matches(&rule_tool_name, &tool_name, behavior);
    }
}
