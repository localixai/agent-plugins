//! Port of `src/loading/resolve/monitor.ts`, switched.
//!
//! A monitor resolved for this host (plugins/manifest-reference.md, "Where each variable
//! resolves"; plugins/components.md, "Monitors").

use indexmap::IndexMap;
use serde::Serialize;

use super::shell_exports::{ExportScope, shell_exports};
use super::shell_guard::reject_user_config_in_shell;
use super::shell_guard_monitor::{MONITOR_GUARD, MonitorGuardContext};
use super::substitute::SubstitutionScope;
use super::substituter::substituter;
use super::surfaces::{ShellPathsScope, SubstitutionSurface, surface_policy};
use crate::format::issues::PluginIssue;
use crate::format::monitors::{Monitor, MonitorTrigger};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// A monitor resolved for this host (plugins/manifest-reference.md, "Where each variable resolves";
/// plugins/components.md, "Monitors"). The kit never starts the process: the host runs `command`
/// through a shell, in the session's working directory, outside any sandbox, with the environment
/// below, and turns what it prints into notifications.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedMonitor {
    /// The monitor's name.
    pub name: String,
    /// The plugin it belongs to, by the id the host uses for it (`SubstitutionScope.pluginId`):
    /// what a host keys a registration by. The same field on every resolved process (judgment,
    /// spec/judgments.md).
    pub plugin_id: String,
    /// `command` with `${...}` substituted.
    pub command: String,
    /// As written: `${...}` does not resolve there (the table lists `command` only).
    pub description: String,
    /// When the monitor starts; `monitorsToStart` decides it from this.
    pub when: MonitorTrigger,
    /// What the process receives besides the host's own environment. The table says "Not
    /// exported" and components.md adds that monitor processes get no
    /// `CLAUDE_PLUGIN_OPTION_<KEY>`: no option and no variable `command` does not name. By default
    /// (`pasted`, Agent Plugins Extended §9.2.4, J9.17) it is empty: the paths are written into
    /// `command`. With the host's opt-in `SubstitutionScope.shellPaths: 'environment-reference'`
    /// it holds the path variables `command` references, so the references have a value
    /// (judgment, spec/judgments.md).
    pub environment: IndexMap<String, String>,
}

/// One monitor resolved.
#[derive(Clone, Debug, Serialize)]
pub struct MonitorResolution {
    /// Undefined when the monitor does not start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<ResolvedMonitor>,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// Resolves one monitor of a plugin. `command` gets the three path variables and `${ENV_VAR}` from
/// the environment, "but never `${user_config.*}`": a command that references it is an error and
/// the monitor does not start. The check reads the template, so it fires with nothing configured.
pub async fn resolve_monitor<L: Locations + Clone, E: Environment>(
    monitor: &Monitor,
    scope: &SubstitutionScope<L, E>,
) -> MonitorResolution {
    let path = format!("monitors.{}.command", monitor.name);
    let context = MonitorGuardContext {
        monitor: monitor.name.clone(),
        plugin: scope.plugin_id.clone(),
    };
    let refused =
        reject_user_config_in_shell(&MONITOR_GUARD, &monitor.command, &context, &path).await;
    if let Some(refused) = refused {
        return MonitorResolution {
            resolved: None,
            issues: vec![refused],
        };
    }
    let run = substituter(scope);
    let shell = ShellPathsScope {
        shell_paths: scope.shell_paths,
    };
    let policy = surface_policy(SubstitutionSurface::MonitorCommand, shell);
    let done = run.run(&monitor.command, &policy, &path).await;
    let environment = shell_exports(
        SubstitutionSurface::MonitorCommand,
        &done.referenced,
        &ExportScope::from(scope),
        &run,
    )
    .await;
    MonitorResolution {
        resolved: Some(ResolvedMonitor {
            name: monitor.name.clone(),
            plugin_id: scope.plugin_id.clone(),
            command: done.text,
            description: monitor.description.clone(),
            when: monitor.when.clone(),
            environment,
        }),
        issues: done.issues,
    }
}

crate::registry::exports! {
    "loading/resolve/monitor" {
        "resolveMonitor" => async ok |monitor: Monitor, scope: SubstitutionScope|
            resolve_monitor(&monitor, &scope).await;
    }
}
