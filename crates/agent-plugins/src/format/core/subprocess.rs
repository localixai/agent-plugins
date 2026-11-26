//! Port of `src/format/core/subprocess.ts`, switched.
//!
//! What a host launches for a core stdio server or hook handler (core §7.2.1, §7.3.3, §9): the
//! command as one token, `./` resolved against the plugin root and a bare name left to the
//! platform's search; the arguments expanded; the working directory (the plugin root unless a
//! server's `cwd` says otherwise; always the root for a handler); and the environment: the host's
//! base, then a server's `env` expanded, then `PLUGIN_ROOT` and `PLUGIN_DATA`, each replacing an
//! entry of the same name by the platform's rule (case-insensitive on Windows).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::command::relative_command_path;
use super::extensions::CoreExtensions;
use super::mcp_server::CoreStdioServer;
use super::placeholders::{CorePaths, CwdResolution, expand_placeholders, resolve_cwd};
use crate::format::path_text::PathPlatform;
use crate::js::{in_key_order, key_order};

/// A hook handler of a core `hooks.json` (`CoreHookHandler` of `src/format/core/hooks.ts`, whose
/// module is not in the core yet).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreHookHandler {
    /// The command token.
    pub command: String,
    /// Its arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Only the namespaces the host implements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<CoreExtensions>,
}

/// What a host launches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreProcess {
    /// The command.
    pub command: String,
    /// Its arguments, expanded.
    pub args: Vec<String>,
    /// Its working directory.
    pub cwd: String,
    /// Its environment.
    pub env: IndexMap<String, String>,
}

/// What a process is made from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreProcessInput {
    /// The resolved plugin root and the instance's data directory (core §9.1).
    pub paths: CorePaths,
    /// The base environment the host chose (core §9.1 lets it inherit, omit or sanitise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<IndexMap<String, String>>,
    /// Whose rules apply to names; `posix` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<PathPlatform>,
}

fn same_name(platform: PathPlatform, first: &str, second: &str) -> bool {
    match platform {
        PathPlatform::Windows => first.to_uppercase() == second.to_uppercase(),
        PathPlatform::Posix => first == second,
    }
}

fn overlay(
    env: &IndexMap<String, String>,
    entries: &[(String, String)],
    platform: PathPlatform,
) -> IndexMap<String, String> {
    let mut result = env.clone();
    for (name, value) in entries {
        let existing: Vec<String> = result
            .keys()
            .filter(|existing| same_name(platform, existing, name))
            .cloned()
            .collect();
        for one in existing {
            result.shift_remove(&one);
        }
        result.insert(name.clone(), value.clone());
        result = in_key_order(result);
    }
    result
}

/// The subprocess environment of core §9.1, `configured` already expanded.
#[must_use]
pub fn core_environment(
    input: &CoreProcessInput,
    configured: &IndexMap<String, String>,
) -> IndexMap<String, String> {
    let platform = input.platform.unwrap_or(PathPlatform::Posix);
    let base = input.base.clone().unwrap_or_default();
    let configured: Vec<(String, String)> = key_order(configured.keys())
        .into_iter()
        .filter_map(|name| {
            configured
                .get(name)
                .map(|value| (name.clone(), value.clone()))
        })
        .collect();
    let own = overlay(&base, &configured, platform);
    let reserved = [
        ("PLUGIN_ROOT".to_owned(), input.paths.root.clone()),
        ("PLUGIN_DATA".to_owned(), input.paths.data.clone()),
    ];
    overlay(&own, &reserved, platform)
}

/// The command a host runs: a `./` path made absolute under the root, a bare name as written.
fn command_of(command: &str, paths: &CorePaths) -> String {
    match relative_command_path(command) {
        Some(relative) => format!("{}/{relative}", paths.root),
        None => command.to_owned(),
    }
}

fn expand_all(values: Option<&[String]>, paths: &CorePaths) -> Vec<String> {
    values
        .unwrap_or_default()
        .iter()
        .map(|value| expand_placeholders(value, paths))
        .collect()
}

/// A hook handler's process (core §7.3.3, §9.2: only `args` is expanded).
#[must_use]
pub fn core_handler_process(handler: &CoreHookHandler, input: &CoreProcessInput) -> CoreProcess {
    CoreProcess {
        command: command_of(&handler.command, &input.paths),
        args: expand_all(handler.args.as_deref(), &input.paths),
        cwd: input.paths.root.clone(),
        env: core_environment(input, &IndexMap::new()),
    }
}

/// A stdio server's process, or why its `cwd` leaves its base once the real paths are known.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CoreServerProcess {
    /// The process.
    Process(CoreProcess),
    /// Why the `cwd` leaves its base.
    Problem(String),
}

/// A stdio server's process (core §7.2.1, §9.2: `args`, `env` values and `cwd` are expanded), or why
/// its `cwd` leaves its base once the real paths are known.
#[must_use]
pub fn core_server_process(
    server: &CoreStdioServer,
    input: &CoreProcessInput,
) -> CoreServerProcess {
    let cwd = server
        .cwd
        .as_deref()
        .map(|configured| resolve_cwd(configured, &input.paths));
    if let Some(CwdResolution::Problem(problem)) = &cwd {
        return CoreServerProcess::Problem(format!("\"cwd\" {problem}"));
    }
    let env: IndexMap<String, String> = server
        .env
        .as_ref()
        .map(|configured| {
            key_order(configured.keys())
                .into_iter()
                .filter_map(|name| configured.get(name).map(|value| (name, value)))
                .map(|(name, value)| (name.clone(), expand_placeholders(value, &input.paths)))
                .collect()
        })
        .unwrap_or_default();
    let cwd_path = match cwd {
        Some(CwdResolution::Cwd(resolved)) => resolved.path,
        Some(CwdResolution::Problem(_)) | None => input.paths.root.clone(),
    };
    CoreServerProcess::Process(CoreProcess {
        command: command_of(&server.command, &input.paths),
        args: expand_all(server.args.as_deref(), &input.paths),
        cwd: cwd_path,
        env: core_environment(input, &env),
    })
}

crate::registry::exports! {
    "format/core/subprocess" {
        "coreEnvironment" => ok |input: CoreProcessInput, configured: Option<IndexMap<String, String>>| core_environment(&input, &configured.unwrap_or_default());
        "coreHandlerProcess" => ok |handler: CoreHookHandler, input: CoreProcessInput| core_handler_process(&handler, &input);
        "coreServerProcess" => ok |server: CoreStdioServer, input: CoreProcessInput| core_server_process(&server, &input);
    }
}
