//! Port of `src/skills/shell-variables.ts`, switched.
//!
//! The text the shell receives for a skill's `!` command (skills.md, "Available string
//! substitutions"; "How injected commands run"). The permission check reads the same command as
//! skill content, its paths written in (`substituteVariables`).

use indexmap::IndexMap;
use serde::Serialize;

use super::variables::{SkillVariables, substitute_pass};
use crate::format::skill_types::SkillShell;
use crate::loading::resolve::shell_exports::{ExportScope, shell_exports};
use crate::loading::resolve::surfaces::{ShellPathsScope, SubstitutionSurface, surface_policy};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::registry::Port;

/// What the shell receives for one `!` command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShellCommandText {
    /// The command.
    pub text: String,
    /// The values of the references `text` holds, for the command's environment.
    pub environment: IndexMap<String, String>,
}

/// The surface whose policy each shell's text is substituted under.
const fn shell_surface(shell: SkillShell) -> SubstitutionSurface {
    match shell {
        SkillShell::Bash => SubstitutionSurface::SkillBash,
        SkillShell::Powershell => SubstitutionSurface::SkillPowershell,
    }
}

/// `${CLAUDE_*}` in a `!` command as its shell receives it: under the scope's `shellPaths`, each path
/// a reference the shell expands from the environment returned with it (judgment, spec/judgments.md,
/// `shellPaths`), or written in when the host asks for `pasted`.
pub async fn substitute_shell_command<L: Locations + Clone, E: Environment>(
    text: &str,
    vars: &SkillVariables<L, E>,
    shell: SkillShell,
) -> ShellCommandText {
    let surface = shell_surface(shell);
    let scope = ShellPathsScope {
        shell_paths: vars.scope.shell_paths,
    };
    let policy = surface_policy(surface, scope);
    let done = substitute_pass(text, vars, &policy, "").await;
    let exports = ExportScope::from(&vars.scope);
    let environment = shell_exports(surface, &done.referenced, &exports, &done.variable).await;
    ShellCommandText {
        text: done.text,
        environment,
    }
}

crate::registry::exports! {
    "skills/shell-variables" {
        "substituteShellCommand" => async ok |text: String, vars: SkillVariables<Port, Port>, shell: SkillShell|
            substitute_shell_command(&text, &vars, shell).await;
    }
}
