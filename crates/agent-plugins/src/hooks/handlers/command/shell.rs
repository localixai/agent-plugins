//! Port of `src/hooks/handlers/command/shell.ts`, switched.
//!
//! The shell a command hook runs in, or none: "A command hook runs as exec form when `args` is set",
//! and exec form has no shell; shell form runs in the hook's `shell`, else the host's default
//! (hooks.md, "Exec form and shell form"; "Command hook fields", `shell`).

use serde::Deserialize;

use crate::js::JsValue;
use crate::ports::command_runner::ShellKind;

/// The fields of a command handler the choice reads.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommandFields {
    /// Exec form's arguments; `undefined` for shell form.
    #[serde(default)]
    pub args: JsValue,
    /// The hook's own `shell`.
    #[serde(default)]
    pub shell: Option<ShellKind>,
}

/// A command handler's registered entry (`HandlerEntry<'command'>`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HandlerEntry {
    /// The handler.
    pub handler: CommandFields,
}

/// What of a run's context the choice reads (`Pick<RunContext, 'defaultShell'>`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellContext {
    /// The host's default shell.
    #[serde(default)]
    pub default_shell: Option<ShellKind>,
}

/// The shell the entry's command runs in, `None` for exec form.
#[must_use]
pub fn shell_of(entry: &HandlerEntry, context: &ShellContext) -> Option<ShellKind> {
    if entry.handler.args.is_undefined() {
        entry.handler.shell.or(context.default_shell)
    } else {
        None
    }
}

crate::registry::exports! {
    "hooks/handlers/command/shell" {
        "shellOf" => ok |entry: HandlerEntry, context: ShellContext| shell_of(&entry, &context);
    }
}
