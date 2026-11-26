//! Port of `src/cli/clean-configuration.ts`, switched.
//!
//! The recipe of debug-your-config.md, "Test against a clean configuration": an empty directory, a
//! fresh `CLAUDE_CONFIG_DIR`, and `--setting-sources user`, so that the session has none of your own
//! configuration; then what each outcome of reproducing the problem points to. Pure: it only builds
//! the launch and the advice, the host or the person runs them.

use serde::{Deserialize, Serialize};

/// The variable that "applies to this one launch" (or, in PowerShell, to the rest of the shell
/// session).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CleanEnvironment {
    /// The fresh configuration directory.
    #[serde(rename = "CLAUDE_CONFIG_DIR")]
    pub claude_config_dir: String,
}

/// The launch of a clean session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanLaunch {
    /// The empty directory to run in.
    pub directory: String,
    /// The fresh configuration directory.
    pub config_dir: String,
    /// The variable that "applies to this one launch" (or, in PowerShell, to the rest of the shell session).
    pub env: CleanEnvironment,
    /// `--setting-sources user` turns off project configuration (`.claude/settings.json`, `CLAUDE.md`, `.mcp.json`).
    pub args: [&'static str; 2],
}

/// The launch for a home directory: `~/claude-clean/empty` and `~/claude-clean/config`.
#[must_use]
pub fn clean_launch(home: &str) -> CleanLaunch {
    let root = format!("{}/claude-clean", home.trim_end_matches(['\\', '/']));
    CleanLaunch {
        directory: format!("{root}/empty"),
        config_dir: format!("{root}/config"),
        env: CleanEnvironment {
            claude_config_dir: format!("{root}/config"),
        },
        args: ["--setting-sources", "user"],
    }
}

/// The documented commands, per shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CleanShell {
    /// A POSIX shell.
    Posix,
    /// PowerShell.
    Powershell,
}

/// The two commands of a clean launch, as one shell writes them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CleanCommands {
    /// Makes the empty directory and goes there.
    pub create: &'static str,
    /// Launches Claude Code with the clean configuration.
    pub launch: &'static str,
}

/// The documented commands, per shell.
#[must_use]
pub const fn clean_launch_commands(shell: CleanShell) -> CleanCommands {
    match shell {
        CleanShell::Posix => CleanCommands {
            create: "mkdir -p ~/claude-clean/empty && cd ~/claude-clean/empty",
            launch: "CLAUDE_CONFIG_DIR=~/claude-clean/config claude --setting-sources user",
        },
        CleanShell::Powershell => CleanCommands {
            create: "New-Item -ItemType Directory -Force \"$HOME/claude-clean/empty\" | Out-Null\nSet-Location \"$HOME/claude-clean/empty\"",
            launch: "$env:CLAUDE_CONFIG_DIR = \"$HOME/claude-clean/config\"\nclaude --setting-sources user",
        },
    }
}

/// What reproducing the problem in the clean session means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CleanOutcome {
    /// The problem is gone.
    ProblemGone,
    /// The problem persists.
    ProblemPersists,
}

/// What a clean run points to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AfterCleanRun {
    /// Where the cause is.
    pub cause: &'static str,
    /// What to do next.
    pub next: Vec<&'static str>,
}

/// "Problem gone": the cause is in your user or project configuration, so copy `~/.claude.json` and the
/// files in `~/.claude` into the clean directory one at a time, then try the project without
/// `--setting-sources user`. "Problem persists": the cause is outside it; check `/status` for managed
/// settings and the shell for environment variables.
#[must_use]
pub fn after_clean_run(outcome: CleanOutcome) -> AfterCleanRun {
    match outcome {
        CleanOutcome::ProblemGone => AfterCleanRun {
            cause: "user-or-project-configuration",
            next: vec![
                "Copy ~/.claude.json and the files in ~/.claude into the clean configuration directory one at a time",
                "If none brings the problem back, start Claude Code in your project directory without --setting-sources user, with CLAUDE_CONFIG_DIR still pointing at the clean directory",
            ],
        },
        CleanOutcome::ProblemPersists => AfterCleanRun {
            cause: "outside-user-and-project-configuration",
            next: vec![
                "Run /status in the session to check for managed settings",
                "Check your shell for environment variables",
            ],
        },
    }
}

crate::registry::exports! {
    "cli/clean-configuration" {
        "cleanLaunch" => ok |home: String| clean_launch(&home);
        "cleanLaunchCommands" => ok |shell: CleanShell| clean_launch_commands(shell);
        "afterCleanRun" => ok |outcome: CleanOutcome| after_clean_run(outcome);
    }
}
