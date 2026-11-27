//! Port of `src/settings/hooks/custom-commands.ts` at sha256:cc1501dec321.
//!
//! The commands that follow the hooks (settings-reference.md, `disableAllHooks` "Status line and
//! file suggestion gates" and `allowManagedHooksOnly`; goal.md, "Requirements"): the status line,
//! the file suggestion command, the subagent status line, plugin monitors and `/goal`. The kit
//! gives the decision; rendering, starting a monitor and running a command stay the host's. Takes
//! the host's `HookGate`: stays TypeScript in the package (the module comment of
//! `crate::settings`), so it exports nothing.

use super::hook_gate::HookGate;

/// "Off entirely" per command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandsDisabled {
    /// The status line.
    pub status_line: bool,
    /// The file suggestion command.
    pub file_suggestion: bool,
    /// The subagent status line.
    pub subagent_status_line: bool,
}

/// The decisions about the commands that follow the hooks.
pub struct CommandGates<'a> {
    /// "Off entirely": managed settings set `disableAllHooks`, or the folder is not trusted under
    /// the workspace trust rule of hooks in settings files. Checked before the narrowing.
    pub commands_disabled: CommandsDisabled,
    /// "Narrowed to managed settings": `allowManagedHooksOnly`, a `disableAllHooks` that is `true`
    /// outside managed settings after precedence, or `--safe-mode`. A managed value runs; any
    /// other is skipped without a warning.
    pub command_settings_narrowed_to_managed: bool,
    /// `/goal` "depends on hooks": not while hooks are disabled, restricted to managed ones, or
    /// untrusted.
    pub goal_command_available: bool,
    /// The settings' decisions about hooks.
    pub gate: &'a dyn HookGate,
}

impl CommandGates<'_> {
    /// A plugin's monitors do not start under `allowManagedHooksOnly` unless managed settings
    /// force-enable the plugin, matched on the whole `plugin@marketplace` id.
    #[must_use]
    pub fn monitors_run(&self, plugin: &str) -> bool {
        !self.gate.managed_only() || self.gate.force_enabled(plugin)
    }
}

/// The facts about the session the host knows and the settings do not.
#[derive(Clone, Copy, Debug, Default)]
pub struct CommandSessionFacts {
    /// The folder is trusted. Defaults to `true`.
    pub folder_trusted: Option<bool>,
    /// Claude Code was started with `--safe-mode`. Defaults to `false`.
    pub safe_mode: Option<bool>,
}

/// The decisions about the commands that follow the hooks.
#[must_use]
pub fn create_command_gates<'a>(
    gate: &'a dyn HookGate,
    session: CommandSessionFacts,
) -> CommandGates<'a> {
    let trusted = session.folder_trusted.unwrap_or(true);
    let off = gate.managed_disable() || !trusted;
    CommandGates {
        commands_disabled: CommandsDisabled {
            status_line: off,
            file_suggestion: off,
            subagent_status_line: off,
        },
        command_settings_narrowed_to_managed: gate.managed_only()
            || gate.other_disable()
            || session.safe_mode.unwrap_or(false),
        goal_command_available: trusted
            && !gate.managed_only()
            && !gate.managed_disable()
            && !gate.other_disable(),
        gate,
    }
}
