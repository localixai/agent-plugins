//! Port of `src/hooks/handlers/remote/mcp-tool.ts`, switched.
//!
//! An `mcp_tool` hook (hooks.md, "MCP tool hook fields"). The `server` is passed as the hook writes
//! it: for a plugin-bundled server that is "the scoped name `plugin:<plugin-name>:<server-name>`,
//! not the bare server key". Running no hook on `SessionStart` at launch and on `Setup` (no MCP
//! client yet) is the select stage's (`select/mcp-client.ts`).

use indexmap::IndexMap;
use serde::Serialize;

use super::failure_run::{EndedRun, EntryEvent, SkippedRun, ended_run, remote_path, without_port};
use super::http::whole_ms;
use super::mcp_input::substitute_input;
use crate::format::env_number::positive_whole_env;
use crate::format::issues::PluginIssue;
use crate::hooks::handlers::context::{RunContext, RunContextRead};
use crate::hooks::handlers::timeouts::{TimedEntry, handler_timeout_ms};
use crate::js::{JsObject, JsValue};
use crate::ports::clock::Clock;
use crate::ports::environment::Environment;
use crate::ports::mcp_caller::{McpAvailability, McpCaller, McpToolCall};
use crate::registry::Port;

/// Whether each event waits for a connecting server, in the order of the TypeScript's table.
const WAITS: [(&str, bool); 33] = [
    ("PreToolUse", true),
    ("PermissionRequest", true),
    ("PermissionDenied", true),
    ("PostToolUse", true),
    ("PostToolBatch", true),
    ("UserPromptSubmit", true),
    ("UserPromptExpansion", true),
    ("Stop", true),
    ("SubagentStop", true),
    ("TeammateIdle", true),
    ("TaskCreated", true),
    ("TaskCompleted", true),
    ("ConfigChange", true),
    ("PreCompact", true),
    ("PreModelSwitch", true),
    ("Elicitation", true),
    ("ElicitationResult", true),
    ("WorktreeCreate", true),
    ("WorktreeRemove", true),
    ("MessageDisplay", true),
    ("SessionStart", false),
    ("Setup", false),
    ("SessionEnd", false),
    ("StopFailure", false),
    ("PostToolUseFailure", false),
    ("SubagentStart", false),
    ("Notification", false),
    ("InstructionsLoaded", false),
    ("CwdChanged", false),
    ("DirectoryAdded", false),
    ("FileChanged", false),
    ("PostCompact", false),
    ("PostModelSwitch", false),
];

/// "On events where a hook can block or change the result, such as `PreToolUse` or `Stop`, Claude
/// Code waits for a connecting server ... On observational events, such as `Notification` or
/// `SessionEnd`, it doesn't wait."
///
/// Judgment (spec/judgments.md): the docs name four events, so the kit sorts the rest by the same
/// test. An event waits when exit code 2 can stop what it guards ("Can block?: Yes" in "Exit code 2
/// behavior per event") or its output decides a result: `PermissionRequest` (`decision`),
/// `PermissionDenied` (`retry`), `PostToolUse` (`updatedToolOutput`, `decision`), `MessageDisplay`
/// (`displayContent`). An event whose hooks only add context or watch (`SessionStart`,
/// `SubagentStart`, `PostToolUseFailure`) does not.
#[must_use]
pub fn waits_for_server() -> IndexMap<&'static str, bool> {
    WAITS.into_iter().collect()
}

/// `MCP_TIMEOUT`: "Timeout in milliseconds for MCP server startup (default: 30000, or 30 seconds)".
const MCP_TIMEOUT_VARIABLE: &str = "MCP_TIMEOUT";
const DEFAULT_MCP_TIMEOUT_MS: f64 = 30_000.0;

async fn mcp_timeout_ms(environment: &impl Environment) -> f64 {
    // J3.17: read as every numeric variable is (spec §3.6.1 item 2).
    let text = environment.get(MCP_TIMEOUT_VARIABLE).await;
    positive_whole_env(text.as_deref()).unwrap_or(DEFAULT_MCP_TIMEOUT_MS)
}

/// "For at most `MCP_TIMEOUT` and within the hook's own `timeout`"; none on an observational event.
async fn wait_ms(event: &str, environment: &impl Environment, hook_ms: Option<f64>) -> f64 {
    if !WAITS.iter().any(|(name, waits)| *name == event && *waits) {
        return 0.0;
    }
    let limit = mcp_timeout_ms(environment).await;
    hook_ms.map_or(limit, |hook| limit.min(hook))
}

/// The hook's `timeout` (none: unlimited) and the clock reading when the hook started.
#[derive(Clone, Copy)]
struct Budget {
    hook_ms: Option<f64>,
    started_at: f64,
}

/// What a run of an `mcp_tool` hook was.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum McpRun {
    /// It did not run.
    Skipped(SkippedRun),
    /// `timed-out` or `cancelled`.
    Ended(EndedRun),
    /// The tool answered, or could not be called.
    Mcp(McpAnswer),
}

/// `{ kind: 'mcp', isError, text }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAnswer {
    /// `mcp`.
    pub kind: &'static str,
    /// Whether the tool reported an error.
    pub is_error: bool,
    /// Its text.
    pub text: String,
}

fn mcp_answer(is_error: bool, text: String) -> McpRun {
    McpRun::Mcp(McpAnswer {
        kind: "mcp",
        is_error,
        text,
    })
}

/// What is left of the hook's `timeout` for the call, "within the hook's own `timeout`" being the
/// wait and the call together. Judgment (spec/judgments.md): a host with no `clock` leaves the call
/// the whole `timeout`, so the wait may add to it.
async fn left_ms(hook_ms: Option<f64>, started_at: f64, clock: Option<Port>) -> Option<f64> {
    match (hook_ms, clock) {
        (Some(hook), Some(clock)) => Some(hook - (clock.now().await - started_at)),
        _ => hook_ms,
    }
}

fn server_of(handler: &JsObject, key: &str) -> String {
    handler
        .get(key)
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// What ends the hook before the tool is called, or `None` when the call goes ahead: a session
/// abort during the wait ("aborted" is the shared `cancelled` run; the port can only answer
/// `unavailable`), a server that is not connected ("the hook produces a non-blocking error"), or a
/// wait that used up the hook's `timeout`.
async fn ended_before_call(
    entry: &TimedEntry,
    context: &RunContextRead,
    client: Port,
    budget: Budget,
) -> Option<McpRun> {
    let server = server_of(&entry.handler, "server");
    let tool = server_of(&entry.handler, "tool");
    let Budget {
        hook_ms,
        started_at,
    } = budget;
    let wait = wait_ms(&entry.event, &context.ports.environment, hook_ms).await;
    let availability = client
        .ensure_connected(&server, whole_ms(wait), &context.signal)
        .await;
    if context.signal.is_aborted() {
        return Some(McpRun::Ended(EndedRun { kind: "cancelled" }));
    }
    if availability == McpAvailability::Unavailable {
        let text = format!(
            "The MCP server \"{server}\" is not connected, so the hook did not call {tool}."
        );
        return Some(mcp_answer(true, text));
    }
    let left = left_ms(hook_ms, started_at, context.ports.clock).await;
    left.filter(|left| *left <= 0.0)
        .map(|_| McpRun::Ended(EndedRun { kind: "timed-out" }))
}

/// The raw run and the issues the run itself found.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct McpHandled {
    /// The run.
    pub run: McpRun,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// Runs one `mcp_tool` hook. `entry` is the registered handler, `context` the run's `RunContext`.
pub async fn run_mcp_tool(entry: &TimedEntry, context: RunContext) -> McpHandled {
    let context = context.read().await;
    let Some(client) = context.ports.mcp else {
        let without = without_port(
            &EntryEvent {
                event: entry.event.clone(),
            },
            "mcp_tool",
            "McpCaller",
        );
        return McpHandled {
            run: McpRun::Skipped(without.run),
            issues: without.issues,
        };
    };
    let server = server_of(&entry.handler, "server");
    let tool = server_of(&entry.handler, "tool");
    let hook_ms = handler_timeout_ms(entry, &context.timeouts);
    let started_at = match context.ports.clock {
        Some(clock) => clock.now().await,
        None => 0.0,
    };
    let budget = Budget {
        hook_ms,
        started_at,
    };
    if let Some(ended) = ended_before_call(entry, &context, client, budget).await {
        return McpHandled {
            run: ended,
            issues: Vec::new(),
        };
    }
    let handler_input = match entry.handler.get("input") {
        Some(JsValue::Object(members)) => Some(members),
        _ => None,
    };
    let event = EntryEvent {
        event: entry.event.clone(),
    };
    let args = substitute_input(handler_input, &context.input, &remote_path(&event, "input"));
    let timeout_ms = left_ms(hook_ms, started_at, context.ports.clock).await;
    let answer = client
        .call_tool(McpToolCall {
            server,
            tool,
            input: args.input,
            timeout_ms: timeout_ms.map(whole_ms),
            signal: context.signal,
        })
        .await;
    let run = match answer {
        Err(failure) => ended_run(&failure)
            .map_or_else(|| mcp_answer(true, failure.message.clone()), McpRun::Ended),
        Ok(result) => mcp_answer(result.is_error, result.text),
    };
    McpHandled {
        run,
        issues: args.issues,
    }
}

crate::registry::exports! {
    "hooks/handlers/remote/mcp-tool" {
        "WAITS_FOR_SERVER" => ok waits_for_server;
        "runMcpTool" => async ok |entry: TimedEntry, context: RunContext| run_mcp_tool(&entry, context).await;
    }
}
