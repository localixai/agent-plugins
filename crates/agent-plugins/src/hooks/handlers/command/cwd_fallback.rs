//! Ported from `src/hooks/handlers/command/cwd-fallback.ts`; only the core uses it now, so it has no wrapper.
//!
//! hooks.md, "Hook handler fields": "Handlers run in the current directory with Claude Code's
//! environment. If the current directory no longer exists, for example a worktree or temp directory
//! that another shell deleted mid-session, Claude Code runs command hooks from the first of these
//! that still exists: the directory the session started in, the project root, your home directory,
//! or the system temp directory. Claude Code records a warning naming the fallback directory."

use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::hooks::handlers::context::{RunContext, RunContextRead};
use crate::ports::locations::Locations;

/// Where a command hook runs, and what that cost.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkingDirectory {
    /// The directory the process starts in.
    pub cwd: String,
    /// The warning that names a fallback.
    pub issues: Vec<PluginIssue>,
}

/// The directory a command hook runs in. `context` is the run's `RunContext`.
pub async fn working_directory(context: RunContext) -> WorkingDirectory {
    let RunContextRead {
        ports,
        cwd,
        start_directory,
        temp_directory,
        signal,
        ..
    } = context.read().await;
    let locations = ports.locations;
    let project_dir = locations.project_dir().await;
    let home_dir = locations.home_dir().await;
    let mut candidates: Vec<String> = Vec::new();
    for directory in [
        Some(cwd.clone()),
        start_directory,
        Some(project_dir),
        Some(home_dir),
        temp_directory,
    ]
    .into_iter()
    .flatten()
    {
        if !candidates.contains(&directory) {
            candidates.push(directory);
        }
    }
    let found = locations
        .existing_directory(&candidates, Some(&signal))
        .await;
    // Judgment (spec/judgments.md): when none of them exists the process is started in `cwd`
    // anyway, and the port reports that it could not start.
    match found {
        Some(found) if found != cwd => {
            let message = format!(
                "The working directory {cwd} no longer exists; the hook runs from {found}."
            );
            WorkingDirectory {
                cwd: found,
                issues: vec![warning("hooks.cwd", &message)],
            }
        }
        _ => WorkingDirectory {
            cwd,
            issues: Vec::new(),
        },
    }
}
