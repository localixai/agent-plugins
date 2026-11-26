//! Port of `src/loading/resolve/lsp-startup.ts`, switched.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::ports::command_runner::CommandSpec;

/// The part of a resolved language server this module reads (`ResolvedLspServer`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLspServer {
    /// The key as the plugin wrote it.
    pub name: String,
    /// `command` with `${...}` substituted.
    pub command: String,
    /// Its arguments.
    pub args: Vec<String>,
    /// The server's own `env`, substituted; empty when it declares none.
    pub env: IndexMap<String, String>,
    /// The variables the process receives besides `env`: the three path variables.
    pub environment: IndexMap<String, String>,
    /// Always `stdio`.
    pub transport: Transport,
    /// The startup timeout.
    #[serde(default)]
    pub startup_timeout_ms: Option<f64>,
    /// The shutdown timeout.
    #[serde(default)]
    pub shutdown_timeout_ms: Option<f64>,
}

/// Always `stdio`: "Claude Code accepts `socket` but runs every server over stdio".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Standard input and output.
    Stdio,
}

/// How to start the process.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspProcessPlan {
    /// The program and its arguments.
    pub spec: CommandSpec,
    /// The working directory.
    pub cwd: String,
    /// The environment.
    pub env: IndexMap<String, String>,
    /// Always `stdio`.
    pub transport: Transport,
    /// The startup timeout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_timeout_ms: Option<f64>,
    /// The shutdown timeout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_timeout_ms: Option<f64>,
}

/// The process to start, and what stopped it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LspStartupPlan {
    /// The process, when the executable is there.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<LspProcessPlan>,
    /// Why there is none.
    pub issues: Vec<PluginIssue>,
}

/// What the host found.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspFacts {
    /// The executable is on `PATH`.
    pub executable_available: bool,
    /// The working directory.
    pub cwd: String,
}

/// Process lookup is a host fact. The kit never downloads or starts the binary.
#[must_use]
pub fn plan_lsp_startup(server: &ResolvedLspServer, facts: &LspFacts) -> LspStartupPlan {
    if !facts.executable_available {
        return LspStartupPlan {
            process: None,
            issues: vec![error(
                &server.name,
                &format!("Executable not found in $PATH: \"{}\"", server.command),
            )],
        };
    }
    let mut env = server.env.clone();
    env.extend(server.environment.clone());
    LspStartupPlan {
        process: Some(LspProcessPlan {
            spec: CommandSpec::Exec {
                command: server.command.clone(),
                args: server.args.clone(),
            },
            cwd: facts.cwd.clone(),
            env,
            transport: server.transport,
            startup_timeout_ms: server.startup_timeout_ms,
            shutdown_timeout_ms: server.shutdown_timeout_ms,
        }),
        issues: Vec::new(),
    }
}

crate::registry::exports! {
    "loading/resolve/lsp-startup" {
        "planLspStartup" => ok |server: ResolvedLspServer, facts: LspFacts| plan_lsp_startup(&server, &facts);
    }
}
