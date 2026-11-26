//! Port of `src/hooks/env-file/events.ts`, switched.

use super::ports::EnvFileKind;
use crate::ports::command_runner::ShellKind;

/// hooks.md, "Persist environment variables": "`CLAUDE_ENV_FILE` is available for SessionStart, Setup,
/// CwdChanged, and FileChanged hooks. Other hook events don't have access to this variable". The
/// variables of `SessionStart` and `Setup` last the session; those of `CwdChanged` and `FileChanged`
/// "persist into subsequent Bash commands until the next CwdChanged event, when Claude Code clears them".
const FILE_OF_EVENT: [(&str, EnvFileKind); 4] = [
    ("SessionStart", EnvFileKind::Session),
    ("Setup", EnvFileKind::Session),
    ("CwdChanged", EnvFileKind::Cwd),
    ("FileChanged", EnvFileKind::Cwd),
];

/// The file the hooks of this event append to, or none for an event that has no `CLAUDE_ENV_FILE`.
#[must_use]
pub fn env_file_kind_of(event: &str) -> Option<EnvFileKind> {
    FILE_OF_EVENT
        .iter()
        .find(|(name, _)| *name == event)
        .map(|(_, kind)| *kind)
}

/// What clears at the start of an event: the `CwdChanged` event clears the file of its own kind.
const CLEARED_BY_EVENT: [(&str, EnvFileKind); 1] = [("CwdChanged", EnvFileKind::Cwd)];

/// The file an event clears, or none.
#[must_use]
pub fn env_file_cleared_by(event: &str) -> Option<EnvFileKind> {
    CLEARED_BY_EVENT
        .iter()
        .find(|(name, _)| *name == event)
        .map(|(_, kind)| *kind)
}

/// "...and neither does a hook that runs in PowerShell, whether through `"shell": "powershell"` or by
/// default on Windows without Git Bash." A hook in exec form runs no shell, so it is not one.
const fn shell_has_file(shell: ShellKind) -> bool {
    match shell {
        ShellKind::Bash => true,
        ShellKind::Powershell => false,
    }
}

/// Whether a hook that runs in `shell` has `CLAUDE_ENV_FILE`.
#[must_use]
pub fn shell_has_env_file(shell: Option<ShellKind>) -> bool {
    shell.is_none_or(shell_has_file)
}

crate::registry::exports! {
    "hooks/env-file/events" {
        "envFileKindOf" => ok |event: String| env_file_kind_of(&event);
    }
}
