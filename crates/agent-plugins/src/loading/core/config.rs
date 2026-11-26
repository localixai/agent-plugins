//! Port of `src/loading/core/config.ts`, switched.
//!
//! `mcp.json` and `hooks.json` of a core package, read from their fixed locations and then held to
//! the filesystem: a `./` command must resolve to a regular file inside the plugin root, and a `cwd`
//! rooted in the plugin root must stay inside it through any link (core §4.1, boundaries 4 and 5).
//! A server or handler that fails is skipped and reported; the rest loads.
//!
//! Judgment (spec/judgments.md): a `./` command that names no file fails command resolution, so the
//! entry is invalid at load (core §7.3.2 item 4), not at launch; a `cwd` that does not exist yet is
//! no load problem, since only its containment is a configuration rule.

use indexmap::IndexMap;
use serde::Serialize;

use super::locations::{FixedJson, fixed_json};
use crate::format::core::command::relative_command_path;
use crate::format::core::hooks::{CORE_HOOK_EVENTS, CoreHookHandler, CoreHooks, read_core_hooks};
use crate::format::core::mcp::{CoreMcpOptions, read_core_mcp};
use crate::format::core::mcp_server::CoreMcpServer;
use crate::format::core::placeholders::{CoreVariable, CwdResolution, resolve_cwd, symbolic_paths};
use crate::format::core::versions::CoreVersion;
use crate::format::issues::{PluginIssue, error};
use crate::format::locate::{LocateContext, locate};
use crate::js::promise_all;
use crate::ports::BoxFuture;

/// Why a `./` command does not resolve inside the root, or `None` (a bare name resolves at launch).
async fn command_issue(context: &LocateContext, place: &str, command: &str) -> Option<String> {
    let relative = relative_command_path(command)?;
    let found = locate(context, place, &relative, None).await;
    if let Some(issue) = found.issue {
        return Some(issue.message);
    }
    if found.stat.is_some_and(|stat| stat.is_file) {
        None
    } else {
        Some(format!(
            "\"command\" {command} names no file in the package"
        ))
    }
}

async fn cwd_issue(context: &LocateContext, place: &str, cwd: Option<&str>) -> Option<String> {
    let cwd = cwd?;
    let CwdResolution::Cwd(resolved) = resolve_cwd(cwd, &symbolic_paths()) else {
        return None;
    };
    if resolved.base != CoreVariable::PluginRoot {
        return None;
    }
    let found = locate(context, place, &resolved.below, None).await;
    found.issue.map(|issue| issue.message)
}

async fn server_issue(
    context: &LocateContext,
    name: &str,
    server: &CoreMcpServer,
) -> Option<String> {
    let CoreMcpServer::Stdio(server) = server else {
        return None;
    };
    let place = format!("mcpServers.{name}");
    if let Some(problem) = command_issue(context, &place, &server.command).await {
        return Some(problem);
    }
    cwd_issue(context, &place, server.cwd.as_deref()).await
}

/// The servers of `mcp.json` that are valid and contained.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CoreMcpRead {
    /// The servers, by name.
    pub servers: IndexMap<String, CoreMcpServer>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// The servers of `mcp.json` that are valid and contained.
pub async fn read_core_mcp_file(
    context: &LocateContext,
    version: CoreVersion,
    options: &CoreMcpOptions,
) -> CoreMcpRead {
    let value = match fixed_json(context, "mcp.json", "MCP is disabled").await {
        FixedJson::Absent { .. } => {
            return CoreMcpRead {
                servers: IndexMap::new(),
                issues: Vec::new(),
            };
        }
        FixedJson::Issue { issue } => {
            return CoreMcpRead {
                servers: IndexMap::new(),
                issues: vec![issue],
            };
        }
        FixedJson::Value { value } => value,
    };
    let parsed = read_core_mcp(&value, version, options);
    let checks: Vec<BoxFuture<'_, Option<String>>> = parsed
        .servers
        .iter()
        .map(|(name, server)| -> BoxFuture<'_, Option<String>> {
            Box::pin(server_issue(context, name, server))
        })
        .collect();
    let problems = promise_all(checks).await;
    let mut servers = IndexMap::new();
    let mut skipped = Vec::new();
    for ((name, server), problem) in parsed.servers.iter().zip(problems) {
        match problem {
            None => {
                servers.insert(name.clone(), server.clone());
            }
            Some(problem) => skipped.push(error(
                &format!("mcpServers.{name}"),
                &format!("Server \"{name}\" is skipped: {problem}"),
            )),
        }
    }
    let mut issues = parsed.issues;
    issues.extend(skipped);
    CoreMcpRead { servers, issues }
}

/// The handlers of `hooks.json` that are valid and whose `./` command resolves inside the root.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CoreHooksRead {
    /// The handlers, by event.
    pub hooks: CoreHooks,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// The handlers of `hooks.json` that are valid and whose `./` command resolves inside the root.
pub async fn read_core_hooks_file(
    context: &LocateContext,
    version: CoreVersion,
    implemented: &[String],
) -> CoreHooksRead {
    let value = match fixed_json(context, "hooks.json", "Hooks are disabled").await {
        FixedJson::Absent { .. } => {
            return CoreHooksRead {
                hooks: IndexMap::new(),
                issues: Vec::new(),
            };
        }
        FixedJson::Issue { issue } => {
            return CoreHooksRead {
                hooks: IndexMap::new(),
                issues: vec![issue],
            };
        }
        FixedJson::Value { value } => value,
    };
    let parsed = read_core_hooks(&value, version, implemented);
    let mut issues = parsed.issues;
    let mut hooks = IndexMap::new();
    for event in CORE_HOOK_EVENTS {
        let handlers = parsed
            .hooks
            .get(event.as_str())
            .cloned()
            .unwrap_or_default();
        let positions = parsed
            .positions
            .get(event.as_str())
            .cloned()
            .unwrap_or_default();
        let mut kept: Vec<CoreHookHandler> = Vec::new();
        for (index, handler) in handlers.into_iter().enumerate() {
            // The handler's index in the file, not among the handlers kept so far (core §7.3.2 item 4).
            let at = positions.get(index).copied().unwrap_or(index);
            let place = format!("hooks.{}[{at}]", event.as_str());
            match command_issue(context, &place, &handler.command).await {
                None => kept.push(handler),
                Some(problem) => {
                    issues.push(error(&place, &format!("Handler is skipped: {problem}")))
                }
            }
        }
        if !kept.is_empty() {
            hooks.insert(event.as_str().to_owned(), kept);
        }
    }
    CoreHooksRead { hooks, issues }
}

crate::registry::exports! {
    "loading/core/config" {
        "readCoreMcpFile" => async ok |context: LocateContext, version: CoreVersion, options: CoreMcpOptions|
            read_core_mcp_file(&context, version, &options).await;
        "readCoreHooksFile" => async ok |context: LocateContext, version: CoreVersion, implemented: Vec<String>|
            read_core_hooks_file(&context, version, &implemented).await;
    }
}
