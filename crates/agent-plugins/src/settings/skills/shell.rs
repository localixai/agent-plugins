//! Port of `src/settings/skills/shell.ts` at sha256:7951e8e3dac9.
//!
//! It takes the host's `PolicyView`, so it stays
//! TypeScript in the package: only its data names are exported.

use serde::{Deserialize, Serialize};

use crate::js::is_truthy;
use crate::settings::layers::PolicyScope;
use crate::settings::view::PolicyView;

/// Where a skill or custom command comes from, for the one question inline shell asks.
pub const SHELL_ORIGINS: [ShellOrigin; 6] = [
    ShellOrigin::User,
    ShellOrigin::Project,
    ShellOrigin::Plugin,
    ShellOrigin::AddDir,
    ShellOrigin::Bundled,
    ShellOrigin::Managed,
];

/// One of `SHELL_ORIGINS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShellOrigin {
    /// The user's.
    User,
    /// The project's.
    Project,
    /// A plugin's.
    Plugin,
    /// An additional directory's.
    AddDir,
    /// Bundled with Claude Code.
    Bundled,
    /// Deployed through managed settings.
    Managed,
}

/// The text that takes the place of an inline command (settings-reference.md, `disableSkillShellExecution`).
pub const SHELL_DISABLED_MARKER: &str = "[shell command execution disabled by policy]";

/// What the expansion does with an inline command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellVerdict {
    /// It runs.
    Run,
    /// It is replaced by `SHELL_DISABLED_MARKER`.
    Replaced,
}

/// Whether the setting reaches an origin: "skills and custom commands from user, project, plugin, or
/// additional-directory sources"; "Bundled skills and skills deployed through managed settings are
/// unaffected".
const fn reached(origin: ShellOrigin) -> bool {
    match origin {
        ShellOrigin::User | ShellOrigin::Project | ShellOrigin::Plugin | ShellOrigin::AddDir => {
            true
        }
        ShellOrigin::Bundled | ShellOrigin::Managed => false,
    }
}

/// `disableSkillShellExecution` after precedence: a `true` in managed settings "can't be overridden by
/// `false` elsewhere"; otherwise the highest file that sets it decides. Unset means inline shell runs.
fn shell_disabled(view: &dyn PolicyView) -> bool {
    let holders = view.each_from_highest("disableSkillShellExecution");
    if holders
        .iter()
        .any(|holder| holder.scope == PolicyScope::Managed && is_truthy(&holder.value))
    {
        return true;
    }
    holders
        .first()
        .is_some_and(|holder| holder.value.as_bool() == Some(true))
}

/// What the skills expansion does with the `` !`cmd` `` and ` ```! ` blocks of a skill from `origin`.
/// Replacing the blocks themselves is the skills expansion's; this is its verdict.
#[must_use]
pub fn shell_verdict(view: &dyn PolicyView, origin: ShellOrigin) -> ShellVerdict {
    if reached(origin) && shell_disabled(view) {
        ShellVerdict::Replaced
    } else {
        ShellVerdict::Run
    }
}

crate::registry::exports! {
    "settings/skills/shell" {
        "SHELL_ORIGINS" => ok || SHELL_ORIGINS;
        "SHELL_DISABLED_MARKER" => ok || SHELL_DISABLED_MARKER;
    }
}
