//! Ported from `src/settings/hooks/custom-commands.ts`; only the core uses it now, so it has no
//! wrapper.
//!
//! The commands that follow the hooks (settings-reference.md, `disableAllHooks` "Status line and
//! file suggestion gates" and `allowManagedHooksOnly`; goal.md, "Requirements"): the status line,
//! the file suggestion command, the subagent status line, plugin monitors and `/goal`. The kit
//! gives the decision; rendering, starting a monitor and running a command stay the host's. The
//! host holds `monitorsRun` as a function of the core's.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::hook_gate::HookGate;
use super::origins::ForceEnabled;
use crate::registry::Function;

/// "Off entirely" per command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandsDisabled {
    /// The status line.
    pub status_line: bool,
    /// The file suggestion command.
    pub file_suggestion: bool,
    /// The subagent status line.
    pub subagent_status_line: bool,
}

/// The decisions about the commands that follow the hooks.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandGates {
    /// "Off entirely": managed settings set `disableAllHooks`, or the folder is not trusted under
    /// the workspace trust rule of hooks in settings files. Checked before the narrowing.
    pub commands_disabled: CommandsDisabled,
    /// "Narrowed to managed settings": `allowManagedHooksOnly`, a `disableAllHooks` that is `true`
    /// outside managed settings after precedence, or `--safe-mode`. A managed value runs; any
    /// other is skipped without a warning.
    pub command_settings_narrowed_to_managed: bool,
    /// A plugin's monitors do not start under `allowManagedHooksOnly` unless managed settings
    /// force-enable the plugin, matched on the whole `plugin@marketplace` id.
    pub monitors_run: Function,
    /// `/goal` "depends on hooks": not while hooks are disabled, restricted to managed ones, or
    /// untrusted.
    pub goal_command_available: bool,
}

/// The facts about the session the host knows and the settings do not.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSessionFacts {
    /// The folder is trusted. Defaults to `true`.
    #[serde(default)]
    pub folder_trusted: Option<bool>,
    /// Claude Code was started with `--safe-mode`. Defaults to `false`.
    #[serde(default)]
    pub safe_mode: Option<bool>,
}

/// `monitorsRun` over the gate.
pub async fn monitors_run(gate: &HookGate, plugin: &str) -> bool {
    !gate.managed_only || gate.force_enabled(plugin).await
}

struct MonitorsRun(Arc<HookGate>);

crate::registry::methods! {
    MonitorsRun {
        "call" => sync async ok |this, plugin: String| monitors_run(&this.0, &plugin).await;
    }
}

/// The decisions about the commands that follow the hooks.
#[must_use]
pub fn create_command_gates(gate: &Arc<HookGate>, session: CommandSessionFacts) -> CommandGates {
    let trusted = session.folder_trusted.unwrap_or(true);
    let off = gate.managed_disable || !trusted;
    CommandGates {
        commands_disabled: CommandsDisabled {
            status_line: off,
            file_suggestion: off,
            subagent_status_line: off,
        },
        command_settings_narrowed_to_managed: gate.managed_only
            || gate.other_disable
            || session.safe_mode.unwrap_or(false),
        monitors_run: Function::new(MonitorsRun(Arc::clone(gate))),
        goal_command_available: trusted
            && !gate.managed_only
            && !gate.managed_disable
            && !gate.other_disable,
    }
}
