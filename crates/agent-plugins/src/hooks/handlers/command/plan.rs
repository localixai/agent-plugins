//! Ported from `src/hooks/handlers/command/plan.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a command hook runs (hooks.md, "Exec form and shell form", "Reference scripts by path"):
//! the form, the placeholders, the checks and the exported variables are `loading/resolve`'s
//! `resolveHookCommand`; this file only says which plugin, which user options and which host
//! defaults a handler gets.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::js::{self, JsValue};
use crate::loading::resolve::hook_command::{
    HookCommandHandler, HookCommandOptions, HookCommandResolution, resolve_hook_command,
};
use crate::loading::resolve::substitute::SubstitutionScope;
use crate::loading::resolve::user_config::ResolvedUserConfig;
use crate::loading::resolve::without_plugin::locations_without_plugin;
use crate::ports::Signal;
use crate::ports::command_runner::{CommandSpec, ShellKind};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::registry::{Callback, Port};

/// The members of a registered `command` handler the plan reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntry {
    /// The event it fires on.
    pub event: String,
    /// The plugin's full id; none for a hook of no plugin.
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// The official handler object.
    pub handler: HookCommandHandler,
}

/// The ports of a run the plan reads.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct PlanPortsRead {
    /// Where the host's directories are.
    pub locations: Port,
    /// The host's environment variables.
    pub environment: Port,
}

/// The members of a `RunContext` the plan reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanContextRead {
    /// The ports.
    pub ports: PlanPortsRead,
    /// The host is Windows.
    pub windows: bool,
    /// The host's default shell.
    #[serde(default)]
    pub default_shell: Option<ShellKind>,
    /// A plugin's saved options; a context that crosses as data holds no function.
    #[serde(skip)]
    pub user_config_of: Option<Callback>,
    /// The session's abort.
    pub signal: Signal,
}

/// A `RunContext` as it crosses: a record that holds a function (`userConfigOf`) is a port whose
/// members are read one by one, one that holds none is data (docs/RUST.md, "Effects").
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum PlanContext {
    /// The context holds a function.
    Host(Port),
    /// The context is data, its ports nested.
    Data(Box<PlanContextRead>),
}

impl PlanContext {
    /// The members the plan reads.
    pub async fn read(self) -> PlanContextRead {
        match self {
            Self::Data(read) => *read,
            Self::Host(port) => PlanContextRead {
                ports: port.member("ports").await,
                windows: port.member("windows").await,
                default_shell: port.member("defaultShell").await,
                user_config_of: port.member("userConfigOf").await,
                signal: port.member("signal").await,
            },
        }
    }
}

/// A spec to run with its exported variables, or none and the issues that say why.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPlan {
    /// What to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<CommandSpec>,
    /// The path variables (and a plugin's options) the process gets on top of its environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exported: Option<IndexMap<String, String>>,
    /// Set when the program would start nothing: the run is a failure to start, with this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_started: Option<String>,
    /// Set when the hook does not run and the conversation view is told why, naming the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// What `${...}` in this handler's `command` and `args` resolve against; `locations` is the
/// host's, or `locationsWithoutPlugin`'s for a hook of no plugin.
async fn substitution_scope<L: Locations>(
    entry: &PlanEntry,
    context: &PlanContextRead,
    locations: L,
) -> SubstitutionScope<L, Port> {
    let plugin_id = entry.plugin_id.clone();
    let user_config = match (&plugin_id, context.user_config_of) {
        (Some(id), Some(user_config_of)) => user_config_of
            .call::<Option<ResolvedUserConfig>>(vec![JsValue::String(id.clone())])
            .await
            .unwrap_or_default(),
        (None | Some(_), _) => ResolvedUserConfig::default(),
    };
    SubstitutionScope {
        plugin_id: plugin_id.unwrap_or_default(),
        locations,
        environment: context.ports.environment,
        user_config,
        windows: context.windows,
        extra: None,
        extra_paths: None,
        shell_paths: None,
        signal: Some(context.signal.clone()),
    }
}

/// "PowerShell hooks and exec-form hooks run without it": only a shell-form Bash hook is wrapped.
const fn prefixed(shell: ShellKind) -> bool {
    match shell {
        ShellKind::Bash => true,
        ShellKind::Powershell => false,
    }
}

/// A POSIX single-quoted word: an embedded `'` is written `'\''`.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// `CLAUDE_CODE_SHELL_PREFIX` (env-vars.md; Agent Plugins Extended §7.3.2.3 item 5, J7.66): a
/// shell-form Bash hook runs as `<prefix> '<command>'`, the whole command line the one shell-quoted
/// argument the wrapper re-runs (`exec bash -c "$1"`); the prefix is written as the user set it.
async fn with_shell_prefix(spec: CommandSpec, environment: &impl Environment) -> CommandSpec {
    let prefix = environment.get("CLAUDE_CODE_SHELL_PREFIX").await;
    let Some(prefix) = prefix.filter(|prefix| !js::trim(prefix).is_empty()) else {
        return spec;
    };
    let CommandSpec::Shell { command, shell } = spec else {
        return spec;
    };
    if !prefixed(shell) {
        return CommandSpec::Shell { command, shell };
    }
    CommandSpec::Shell {
        command: format!("{prefix} {}", quoted(&command)),
        shell,
    }
}

/// `resolveHookCommand` over this handler's scope.
async fn resolve<L: Locations + Clone>(
    entry: &PlanEntry,
    context: &PlanContextRead,
    locations: L,
) -> HookCommandResolution {
    let scope = substitution_scope(entry, context, locations).await;
    let options = HookCommandOptions {
        path: Some(format!("hooks.{}", entry.event)),
        default_shell: context.default_shell,
        of_plugin: Some(entry.plugin_id.is_some()),
        event: Some(entry.event.clone()),
    };
    resolve_hook_command(&entry.handler, &scope, &options).await
}

/// The plan of one command handler; issues are on `hooks.<event>.command` and `.args[i]`.
pub async fn plan_command(entry: &PlanEntry, context: &PlanContextRead) -> CommandPlan {
    let locations = context.ports.locations;
    let HookCommandResolution {
        resolved,
        not_started,
        notice,
        issues,
    } = match entry.plugin_id {
        None => resolve(entry, context, locations_without_plugin(locations)).await,
        Some(_) => resolve(entry, context, locations).await,
    };
    let (spec, exported) = match resolved {
        None => (None, None),
        Some(resolved) => {
            let spec = with_shell_prefix(resolved.spec, &context.ports.environment).await;
            let exported = resolved
                .environment
                .into_iter()
                .filter_map(|(name, value)| value.as_str().map(|text| (name, text.to_owned())))
                .collect();
            (Some(spec), Some(exported))
        }
    };
    CommandPlan {
        spec,
        exported,
        not_started,
        notice,
        issues,
    }
}
