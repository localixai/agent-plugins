//! Port of `src/hooks/handlers/command/env.ts`, switched.
//!
//! The environment of a command hook (hooks.md, "Common input fields", "Exec form and shell form"
//! and "Persist environment variables"; env-vars.md; plugins/components.md, "Environment, quoting,
//! and matching MCP tools").
//!
//! - "A hook process inherits the parent environment, apart from the `OTEL_*` exporter variables
//!   that Claude Code removes from every subprocess it spawns". The parent environment is the
//!   host's with the settings files' `env` written over it (env-vars.md, "Precedence"), and with
//!   its credentials removed when `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` is `1` (`scrub.ts`).
//! - Both forms "export [the path placeholders] as the environment variables `CLAUDE_PROJECT_DIR`,
//!   `CLAUDE_PLUGIN_ROOT`, and `CLAUDE_PLUGIN_DATA`"; a plugin hook also gets
//!   `CLAUDE_PLUGIN_OPTION_<KEY>` for each option.
//! - What Claude Code sets in every subprocess it spawns: `CLAUDECODE` and
//!   `CLAUDE_CODE_CHILD_SESSION` (`1`), `CLAUDE_CODE_SESSION_ID` (the input's `session_id`) and
//!   `CLAUDE_PID` (its own process id).
//! - `CLAUDE_ENV_FILE` on the four events that have one, and only to a hook that runs no
//!   PowerShell; an inherited one never reaches another hook ("Other hook events don't have access
//!   to this variable").
//!
//! Judgment (spec/judgments.md): the exported variables replace an inherited one of the same name,
//! and the `OTEL_` prefix and the `CLAUDE_ENV_FILE` name are matched without regard to case on a
//! Windows host, whose environment names are case-insensitive, and exactly elsewhere.

use indexmap::IndexMap;
use serde::Deserialize;

use super::shell::{HandlerEntry, ShellContext, shell_of};
use super::subprocess_env::{SubprocessEnvironmentInput, SubprocessKind, subprocess_environment};
use crate::js::value::to_js_value;
use crate::js::{JsValue, in_key_order};
use crate::ports::Signal;
use crate::ports::command_runner::ShellKind;
use crate::ports::environment::Environment;
use crate::registry::Port;

const ENV_FILE: &str = "CLAUDE_ENV_FILE";

/// The ports of a run this module reads.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct EnvPortsRead {
    /// The host's environment variables.
    pub environment: Port,
}

/// The members of a `RunContext` the environment is built from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvContextRead {
    /// The ports.
    pub ports: EnvPortsRead,
    /// The host's variable names are case-insensitive.
    pub windows: bool,
    /// The host's default shell.
    #[serde(default)]
    pub default_shell: Option<ShellKind>,
    /// The session's id.
    #[serde(default)]
    pub session_id: Option<String>,
    /// The host process's id.
    #[serde(default)]
    pub process_id: Option<f64>,
    /// The effort level.
    #[serde(default)]
    pub effort: Option<String>,
    /// The HIPAA configuration applies.
    #[serde(default)]
    pub hipaa: Option<bool>,
    /// The variables of the settings files' `env` blocks.
    #[serde(default)]
    pub settings_env: Option<IndexMap<String, String>>,
    /// The session's `CLAUDE_ENV_FILE` files (`HookEnvFiles`), the host's object.
    #[serde(default)]
    pub env_files: Option<Port>,
    /// The session's abort.
    pub signal: Signal,
}

/// A `RunContext` as it crosses: a record that holds a function is a port whose members are read
/// one by one, one that holds none is data (docs/RUST.md, "Effects").
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum EnvContext {
    /// The context holds a function.
    Host(Port),
    /// The context is data, its ports nested.
    Data(Box<EnvContextRead>),
}

impl EnvContext {
    /// The members the environment is built from.
    pub async fn read(self) -> EnvContextRead {
        match self {
            Self::Data(read) => *read,
            Self::Host(port) => EnvContextRead {
                ports: port.member("ports").await,
                windows: port.member("windows").await,
                default_shell: port.member("defaultShell").await,
                session_id: port.member("sessionId").await,
                process_id: port.member("processId").await,
                effort: port.member("effort").await,
                hipaa: port.member("hipaa").await,
                settings_env: port.member("settingsEnv").await,
                env_files: port.member("envFiles").await,
                signal: port.member("signal").await,
            },
        }
    }
}

/// The `CLAUDE_ENV_FILE` of this hook: the file of its event, or none.
async fn env_file_variable(entry: &JsValue, context: &EnvContextRead) -> IndexMap<String, String> {
    let Some(env_files) = context.env_files else {
        return IndexMap::new();
    };
    let handler = HandlerEntry::deserialize(entry.clone()).unwrap_or_default();
    let shell = shell_of(
        &handler,
        &ShellContext {
            default_shell: context.default_shell,
        },
    );
    let event = entry.get("event").cloned().unwrap_or_default();
    let args = vec![
        event,
        // `undefined` for exec form, as the package passes it, never `null`.
        shell.map_or(JsValue::Undefined, |kind| {
            to_js_value(&kind).unwrap_or_default()
        }),
        to_js_value(&context.signal).unwrap_or_default(),
    ];
    let env_file = env_files.method::<Option<String>>("pathFor", args).await;
    env_file
        .map(|path| IndexMap::from([(ENV_FILE.to_owned(), path)]))
        .unwrap_or_default()
}

/// The whole environment the process gets (the port adds nothing and removes nothing);
/// `exported` is what `resolveHookCommand` exports, resolved with the command's own substituter.
pub async fn process_environment(
    entry: &JsValue,
    exported: &IndexMap<String, String>,
    context: &EnvContextRead,
) -> IndexMap<String, String> {
    let mut environment = subprocess_environment(&SubprocessEnvironmentInput {
        kind: SubprocessKind::Hook,
        environment: context.ports.environment.snapshot().await,
        settings_env: context.settings_env.clone(),
        windows: context.windows,
        session_id: context.session_id.clone(),
        process_id: context.process_id,
        effort: context.effort.clone(),
        hipaa: context.hipaa,
    });
    let env_file = env_file_variable(entry, context).await;
    for (name, value) in exported.iter().chain(&env_file) {
        environment.insert(name.clone(), value.clone());
    }
    in_key_order(environment)
}

crate::registry::exports! {
    "hooks/handlers/command/env" {
        "processEnvironment" => async ok
            |entry: JsValue, exported: IndexMap<String, String>, context: EnvContext|
            process_environment(&entry, &exported, &context.read().await).await;
    }
}
