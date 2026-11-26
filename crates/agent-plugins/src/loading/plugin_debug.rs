//! Port of `src/loading/plugin-debug.ts`, switched.
//!
//! The checks of "Test and debug" (plugins/create.md): "When a change to your plugin doesn't show up,
//! work through these checks in order. Each one tells you what Claude Code did with the plugin." The
//! list only; running the commands is the cli stream's and the host's.

use serde::{Deserialize, Serialize};

/// Where the command is typed: "In your shell" or "in the running session".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DebugPlace {
    /// In your shell.
    Shell,
    /// In the running session.
    Session,
}

/// One check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebugStep {
    /// Where it is typed.
    pub place: DebugPlace,
    /// What is typed.
    pub command: String,
    /// What the step tells about the plugin.
    pub tells: &'static str,
}

/// What the host knows of the plugin.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugFacts {
    /// The plugin sits in a skills directory (`~/.claude/skills/` or a project's `.claude/skills/`).
    pub in_skills_directory: bool,
    /// The plugin's folder, for a plugin that is not installed: `claude plugin list` takes it as `--plugin-dir`.
    #[serde(default)]
    pub plugin_dir: Option<String>,
}

fn validate() -> DebugStep {
    DebugStep {
        place: DebugPlace::Shell,
        command: "claude plugin validate <path>".to_owned(),
        tells: "whether the manifest and the frontmatter of every skill, agent and command file are valid",
    }
}

fn reload() -> DebugStep {
    DebugStep {
        place: DebugPlace::Session,
        command: "/reload-plugins".to_owned(),
        tells: "applies the edits made on disk and prints one `Reloaded:` line with counts",
    }
}

fn manager() -> DebugStep {
    DebugStep {
        place: DebugPlace::Session,
        command: "/plugin".to_owned(),
        tells: "the Installed tab lists the plugin and the components found; the Errors tab lists what failed to load and why",
    }
}

/// `claude plugin list` shows session-only plugins "only when the same flag precedes the subcommand".
fn list(plugin_dir: Option<&str>) -> DebugStep {
    DebugStep {
        place: DebugPlace::Shell,
        command: match plugin_dir {
            None => "claude plugin list".to_owned(),
            Some(directory) => format!("claude --plugin-dir {directory} plugin list"),
        },
        tells: "prints session-only and skills-directory plugins in their own sections with `Status: ✔ loaded` or the load error",
    }
}

fn blocked() -> DebugStep {
    DebugStep {
        place: DebugPlace::Session,
        command: "/status".to_owned(),
        tells: "a skills-directory plugin that `claude plugin list` does not print, with no error there or in `/plugin`, may be blocked by managed settings",
    }
}

/// The ordered checks; the last applies only to a plugin in a skills directory.
#[must_use]
pub fn plugin_debug_steps(facts: &DebugFacts) -> Vec<DebugStep> {
    let mut steps = vec![
        validate(),
        reload(),
        manager(),
        list(facts.plugin_dir.as_deref()),
    ];
    if facts.in_skills_directory {
        steps.push(blocked());
    }
    steps
}

/// The checks of the other components: `/mcp` for a server, the debug log for a hook.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentChecks {
    /// `/mcp`: a healthy server is connected.
    pub mcp_server: ComponentCheck,
    /// Trigger the event, then read the debug log.
    pub hook: ComponentCheck,
}

/// One of the `COMPONENT_CHECKS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ComponentCheck {
    /// Where it is typed.
    pub place: DebugPlace,
    /// What is typed.
    pub command: &'static str,
    /// What the step tells about the plugin.
    pub tells: &'static str,
}

/// The checks of the other components: `/mcp` for a server, the debug log for a hook.
pub const COMPONENT_CHECKS: ComponentChecks = ComponentChecks {
    mcp_server: ComponentCheck {
        place: DebugPlace::Session,
        command: "/mcp",
        tells: "lists a healthy server as connected",
    },
    hook: ComponentCheck {
        place: DebugPlace::Session,
        command: "trigger the event the hook matches, then read the debug log",
        tells: "which hooks matched, their exit codes and their output",
    },
};

crate::registry::exports! {
    "loading/plugin-debug" {
        "pluginDebugSteps" => ok |facts: DebugFacts| plugin_debug_steps(&facts);
        "COMPONENT_CHECKS" => ok || COMPONENT_CHECKS;
    }
}
