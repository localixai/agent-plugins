//! Port of `src/loading/resolve/shell-guard-hook.ts`, switched.
//!
//! A shell-form hook `command`, which runs through a shell (hooks.md, "Exec form and shell form";
//! errors.md, "Plugin command references user_config in a shell command"). The message is the one
//! errors.md quotes for a hook: `Hook from plugin <plugin> references ${user_config.*} in a
//! shell-form command`, the way out, and the offending template after `Command:`.
//!
//! Judgment: `<plugin>` is the plugin identifier the host uses (`name@marketplace`, or the bare
//! name), the form errors.md prints (`formatter@acme-tools`); `<command>` is the template as written.

use serde::Deserialize;

use crate::registry::Handle;

/// What the hook's message names.
#[derive(Clone, Debug, Deserialize)]
pub struct HookGuardContext {
    /// The plugin identifier.
    pub plugin: String,
    /// The template as written.
    pub command: String,
}

/// The hook's guard (`ShellGuard`): its documented message.
#[derive(Debug)]
pub struct HookGuard;

impl HookGuard {
    /// The documented message of the surface, quoted from errors.md.
    #[must_use]
    pub fn message(&self, context: &HookGuardContext) -> String {
        let HookGuardContext { plugin, command } = context;
        format!(
            "Hook from plugin {plugin} references ${{user_config.*}} in a shell-form command. The substituted value would be re-parsed by the shell. Use exec form instead — {{\"command\": \"<executable>\", \"args\": [\"${{user_config.KEY}}\", ...]}} — or read $CLAUDE_PLUGIN_OPTION_<KEY> from the hook's environment. Command: {command}"
        )
    }
}

crate::registry::methods! {
    HookGuard {
        "message" => ok |this, context: HookGuardContext| this.message(&context);
    }
}

/// The guard of a shell-form hook command.
pub const HOOK_GUARD: HookGuard = HookGuard;

crate::registry::exports! {
    "loading/resolve/shell-guard-hook" {
        "HOOK_GUARD" => ok || Handle::new(HOOK_GUARD);
    }
}
