//! Port of `src/settings/customization/preapproval.ts`, switched.

use std::collections::HashSet;

use serde::Deserialize;

use crate::registry::Set;

/// Where a skill comes from, as far as `allowed-tools` pre-approval is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreapprovalSource {
    /// A personal skill.
    User,
    /// A project skill.
    Project,
    /// A skill of an added directory.
    AddDir,
    /// A skill synced from claude.ai.
    Synced,
    /// A plugin's skill.
    Plugin,
    /// A skill deployed through managed settings.
    Managed,
    /// A bundled skill.
    Bundled,
}

/// What the host knows about the skill that is about to run.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreapprovalSkill {
    /// Where the skill comes from.
    pub source: PreapprovalSource,
    /// The skill's frontmatter declares `hooks`.
    pub declares_hooks: bool,
    /// The skill runs in a subagent through `context: fork`.
    pub context_fork: bool,
}

/// Whose frontmatter hooks the lock stops, so whose pre-approval it takes: "a personal or project
/// skill" (settings-reference.md, `strictPluginOnlyCustomization.hooks`). The docs name no other
/// source, so a skill from an added directory or claude.ai keeps it (spec/judgments.md).
const fn loses(source: PreapprovalSource) -> bool {
    match source {
        PreapprovalSource::User | PreapprovalSource::Project => true,
        PreapprovalSource::AddDir
        | PreapprovalSource::Synced
        | PreapprovalSource::Plugin
        | PreapprovalSource::Managed
        | PreapprovalSource::Bundled => false,
    }
}

/// Whether the tools the skill's `allowed-tools` lists are pre-approved while it runs. Under the
/// `hooks` lock a personal or project skill that declares hooks loses it, unless it runs in a
/// subagent through `context: fork`; a skill that declares no hooks keeps it too
/// (settings-reference.md, `strictPluginOnlyCustomization.hooks`; skills.md, "When managed
/// settings lock hooks to plugins").
#[must_use]
pub fn allowed_tools_preapproved(locked: &HashSet<String>, skill: &PreapprovalSkill) -> bool {
    !(locked.contains("hooks")
        && loses(skill.source)
        && skill.declares_hooks
        && !skill.context_fork)
}

crate::registry::exports! {
    "settings/customization/preapproval" {
        "allowedToolsPreapproved" => ok |locked: Set<String>, skill: PreapprovalSkill| allowed_tools_preapproved(&locked.members.into_iter().collect(), &skill);
    }
}
