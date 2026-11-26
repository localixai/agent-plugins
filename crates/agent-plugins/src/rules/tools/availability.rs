//! Port of `src/rules/tools/availability.ts`, switched.
//!
//! The facts of a session that decide which of the tools the host can provide Claude actually
//! sees. The kit holds no model, provider or surface table: the candidates a host passes are the
//! tools its model and surface offer (tools-reference.md, "Task tool availability",
//! "EndConversation tool behavior"), and this narrows them by the documented rules
//! (spec/judgments.md).

use indexmap::IndexSet;
use serde::Deserialize;

use super::host_tool_names::{HostToolNamesGiven, tool_names_of};
use super::names::{is_mcp_tool_name, tool_name_of_entry};
use crate::rules::tool_pattern::tool_name_matches;
use crate::rules::types::{PermissionBehavior, PermissionRule, Platform};

/// The session's facts (`ToolSession`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSession {
    /// WSL counts as `posix`: Glob and Grep are absent by default everywhere but Windows.
    pub platform: Platform,
    /// Every deny and allow rule in force, whatever its source, including `--disallowedTools` and
    /// `--allowedTools` entries.
    pub rules: Vec<PermissionRule>,
    /// The `--tools` list: "restricts the session's built-in tools to the ones it names".
    #[serde(default)]
    pub tools_flag: Option<Vec<String>>,
    /// The `--allowedTools` entries (rule syntax), for the one thing they do to availability:
    /// restore Glob and Grep.
    #[serde(default)]
    pub allowed_tools_flag: Option<Vec<String>>,
    /// A `--restricted` session removes the built-in tools that run commands or code and
    /// `WebFetch`, unless `--tools` names them individually (cli-reference.md, `--restricted`).
    #[serde(default)]
    pub restricted: Option<bool>,
    /// Windows with Git Bash installed: a Bash deny rule then turns the PowerShell tool off.
    #[serde(default)]
    pub git_bash: Option<bool>,
    /// `CLAUDE_CODE_USE_POWERSHELL_TOOL=1`, in the environment or a settings file's `env` block.
    #[serde(default)]
    pub power_shell_opt_in: Option<bool>,
    /// `permissions.blockWritesOutsideWorkingDirectories` is on: the worktree tools are removed.
    #[serde(default)]
    pub block_writes_outside_working_directories: Option<bool>,
    /// `permissions.denyInsteadOfPrompting` is on: the question and plan tools are left out.
    #[serde(default)]
    pub deny_instead_of_prompting: Option<bool>,
    /// The session is a teammate in an agent team: it keeps the plan tools under
    /// `denyInsteadOfPrompting`.
    #[serde(default)]
    pub teammate: Option<bool>,
    /// How the candidates' names correspond to Claude Code's, when the host's tools carry other
    /// names. Absent, the candidates are Claude Code's names.
    #[serde(default)]
    pub names: Option<HostToolNamesGiven>,
}

const END_CONVERSATION: &str = "EndConversation";
const SEARCH_TOOLS: [&str; 2] = ["Glob", "Grep"];
const POWERSHELL: &str = "PowerShell";
const WORKTREE_TOOLS: [&str; 2] = ["EnterWorktree", "ExitWorktree"];
/// The tools that wait on a person's answer, which `denyInsteadOfPrompting` leaves out.
const QUESTION_TOOLS: [&str; 1] = ["AskUserQuestion"];
const PLAN_TOOLS: [&str; 2] = ["EnterPlanMode", "ExitPlanMode"];
/// The built-in tools `--restricted` removes: the ones that run commands or code, and WebFetch
/// (spec/judgments.md).
const RESTRICTED_TOOLS: [&str; 4] = ["Bash", POWERSHELL, "Monitor", "WebFetch"];

/// "A bare tool name ... removes the tool from Claude's context": a deny rule with no specifier.
fn removes_tool(rule: &PermissionRule, tool_name: &str) -> bool {
    rule.behavior == PermissionBehavior::Deny
        && rule.specifier.is_none()
        && tool_name_matches(rule, tool_name)
}

fn names_tool(entries: Option<&Vec<String>>, tool_name: &str) -> bool {
    entries.is_some_and(|entries| {
        entries
            .iter()
            .any(|entry| tool_name_of_entry(entry) == tool_name)
    })
}

const fn on(flag: Option<bool>) -> bool {
    matches!(flag, Some(true))
}

/// `--restricted` removes the tool: it is one of the restricted ones and `--tools` does not name
/// it individually.
fn restricted_away(tool_name: &str, session: &ToolSession) -> bool {
    on(session.restricted)
        && RESTRICTED_TOOLS.contains(&tool_name)
        && !names_tool(session.tools_flag.as_ref(), tool_name)
}

/// Bash is gone from the session: a bare deny rule or `--restricted` removes it
/// (tools-reference.md, "Glob tool behavior").
fn bash_removed(session: &ToolSession) -> bool {
    restricted_away("Bash", session) || session.rules.iter().any(|rule| removes_tool(rule, "Bash"))
}

/// tools-reference.md, "Glob tool behavior": on macOS, Linux and WSL the tools are left out
/// unless `--tools` names them, naming either in `--allowedTools` restores both ("An allow rule
/// in a settings file doesn't have this effect"), or a deny rule, `--disallowedTools` or
/// `--restricted` removes Bash. A tool that is not one of the two is always offered.
fn offered_by_default(tool_name: &str, session: &ToolSession) -> bool {
    if !SEARCH_TOOLS.contains(&tool_name) || session.platform == Platform::Windows {
        return true;
    }
    bash_removed(session)
        || names_tool(session.tools_flag.as_ref(), tool_name)
        || SEARCH_TOOLS
            .iter()
            .any(|search| names_tool(session.allowed_tools_flag.as_ref(), search))
}

/// settings-reference.md, "What else `denyInsteadOfPrompting` stops": "Claude Code leaves out
/// `AskUserQuestion`, `EnterPlanMode`, and `ExitPlanMode` ... A teammate in an agent team keeps
/// `EnterPlanMode` and `ExitPlanMode`".
fn left_out_unprompted(tool_name: &str, session: &ToolSession) -> bool {
    on(session.deny_instead_of_prompting)
        && (QUESTION_TOOLS.contains(&tool_name)
            || (PLAN_TOOLS.contains(&tool_name) && !on(session.teammate)))
}

/// `--tools` restricts built-in tools only: an MCP tool is not on that list (spec/judgments.md).
/// The entry `default` is the default set, which restricts nothing (cli-reference.md, `--tools`).
fn in_tools_flag(tool_name: &str, session: &ToolSession) -> bool {
    session.tools_flag.is_none()
        || is_mcp_tool_name(tool_name)
        || names_tool(session.tools_flag.as_ref(), "default")
        || names_tool(session.tools_flag.as_ref(), tool_name)
}

/// tools-reference.md, "Bash deny rules also turn off the PowerShell tool": on Windows with Git
/// Bash, any deny rule on Bash, scoped or not, turns PowerShell off, unless
/// `CLAUDE_CODE_USE_POWERSHELL_TOOL=1` or a scoped `PowerShell(...)` rule is set.
fn power_shell_off_by_bash_deny(session: &ToolSession) -> bool {
    if session.platform != Platform::Windows || !on(session.git_bash) {
        return false;
    }
    if on(session.power_shell_opt_in) {
        return false;
    }
    let scoped_power_shell = session
        .rules
        .iter()
        .any(|rule| rule.specifier.is_some() && tool_name_matches(rule, POWERSHELL));
    !scoped_power_shell
        && session.rules.iter().any(|rule| {
            rule.behavior == PermissionBehavior::Deny && tool_name_matches(rule, "Bash")
        })
}

/// `--tools ""` with no MCP tool left and no allow rule naming `EndConversation`, or every other
/// tool denied along with it.
fn end_conversation_dropped(others: &[String], session: &ToolSession) -> bool {
    let allowed_by_name = session.rules.iter().any(|rule| {
        rule.behavior == PermissionBehavior::Allow
            && rule.specifier.is_none()
            && !rule.tool_glob
            && rule.value.tool_name == END_CONVERSATION
    });
    if allowed_by_name {
        return false;
    }
    let no_mcp_left = !others.iter().any(|other| is_mcp_tool_name(other));
    let empty_tools_flag = session.tools_flag.as_ref().is_some_and(Vec::is_empty) && no_mcp_left;
    let denied_too = session
        .rules
        .iter()
        .any(|rule| removes_tool(rule, END_CONVERSATION));
    empty_tools_flag || (others.is_empty() && denied_too)
}

fn kept(tool_name: &str, session: &ToolSession) -> bool {
    offered_by_default(tool_name, session)
        && in_tools_flag(tool_name, session)
        && !restricted_away(tool_name, session)
        && !(tool_name == POWERSHELL && power_shell_off_by_bash_deny(session))
        && !(WORKTREE_TOOLS.contains(&tool_name)
            && on(session.block_writes_outside_working_directories))
        && !left_out_unprompted(tool_name, session)
        && !session
            .rules
            .iter()
            .any(|rule| removes_tool(rule, tool_name))
}

/// The Claude Code tools Claude sees, from Claude Code candidates (order kept).
fn available_claude_tools(candidates: &[String], session: &ToolSession) -> Vec<String> {
    let others: Vec<String> = candidates
        .iter()
        .filter(|tool_name| *tool_name != END_CONVERSATION && kept(tool_name, session))
        .cloned()
        .collect();
    let drops_end = end_conversation_dropped(&others, session);
    candidates
        .iter()
        .filter(|tool_name| {
            if *tool_name == END_CONVERSATION {
                !drops_end
            } else {
                others.contains(tool_name)
            }
        })
        .cloned()
        .collect()
}

/// The tools Claude sees, from the candidates the host can provide (order kept). A deny rule with
/// no specifier removes the tools it names, and so does a glob (permissions.md, "Manage
/// permissions", "Tool name wildcards"); a scoped or parameter rule, an ask rule and an allow rule
/// remove nothing. `EndConversation` is the exception (tools-reference.md, "EndConversation tool
/// behavior"): no deny rule and no `--tools` list removes it while another tool remains; `--tools
/// ""` removes it "only when no MCP tools remain" (cli-reference.md); when the deny rules remove
/// every other tool and also match it, as `*` does, it goes too. An allow rule naming
/// `EndConversation` explicitly saves it in both cases. With `session.names` the candidates are
/// the host's tools, read through their Claude Code names (judgment, spec/judgments.md).
pub async fn available_tools(candidates: &[String], session: ToolSession) -> Vec<String> {
    let given = match session.names.clone() {
        Some(given) => Some(given.read().await),
        None => None,
    };
    let names = tool_names_of(given);
    let claude: IndexSet<String> = candidates
        .iter()
        .flat_map(|candidate| names.claude_names(candidate))
        .collect();
    let claude: Vec<String> = claude.into_iter().collect();
    let kept: IndexSet<String> = available_claude_tools(&claude, &session)
        .into_iter()
        .collect();
    // A host tool is one grant: it goes when any of its Claude Code names is removed, as a denied
    // name takes a subagent's host tool (`resolveSubagentTools`).
    candidates
        .iter()
        .filter(|tool| {
            names
                .claude_names(tool)
                .iter()
                .all(|name| kept.contains(name))
        })
        .cloned()
        .collect()
}

crate::registry::exports! {
    "rules/tools/availability" {
        "availableTools" => async ok |candidates: Vec<String>, session: ToolSession| available_tools(&candidates, session).await;
    }
}
