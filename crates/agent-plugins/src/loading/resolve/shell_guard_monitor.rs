//! Port of `src/loading/resolve/shell-guard-monitor.ts`, switched.
//!
//! A monitor `command`, which runs through a shell (plugins/components.md, "Monitors"; errors.md,
//! "Plugin command references user_config in a shell command"). The message is the one errors.md
//! quotes for a monitor: `Monitor "<name>" from plugin <plugin> references ${user_config.*} in its
//! command` followed by the explanation.
//!
//! Judgment: `<plugin>` is the plugin identifier the host uses (`name@marketplace`, or the bare name),
//! the form the shell-form hook message of the same page shows (`formatter@acme-tools`); the monitor
//! example prints a name with no marketplace, which a bare identifier gives too.

use serde::Deserialize;

use crate::registry::Handle;

/// What the monitor's message names.
#[derive(Clone, Debug, Deserialize)]
pub struct MonitorGuardContext {
    /// The monitor's name.
    pub monitor: String,
    /// The plugin identifier.
    pub plugin: String,
}

/// The monitor's guard (`ShellGuard`): its documented message.
#[derive(Debug)]
pub struct MonitorGuard;

impl MonitorGuard {
    /// The documented message of the surface, quoted from errors.md.
    #[must_use]
    pub fn message(&self, context: &MonitorGuardContext) -> String {
        let MonitorGuardContext { monitor, plugin } = context;
        format!(
            "Monitor \"{monitor}\" from plugin {plugin} references ${{user_config.*}} in its command. The substituted value would be passed to a shell. Monitor commands cannot safely reference ${{user_config.*}}; have the monitor script read the value from a config file or prompt instead."
        )
    }
}

crate::registry::methods! {
    MonitorGuard {
        "message" => ok |this, context: MonitorGuardContext| this.message(&context);
    }
}

/// The guard of a monitor command.
pub const MONITOR_GUARD: MonitorGuard = MonitorGuard;

crate::registry::exports! {
    "loading/resolve/shell-guard-monitor" {
        "MONITOR_GUARD" => ok || Handle::new(MONITOR_GUARD);
    }
}
