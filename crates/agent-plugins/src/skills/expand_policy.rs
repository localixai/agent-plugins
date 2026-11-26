//! Port of `src/skills/expand-policy.ts`, switched.
//!
//! The managed settings that take a skill's `allowed-tools` or `hooks` away (skills.md, "When
//! managed settings lock hooks to plugins" and "When only managed permission rules apply"). The host
//! reads the settings; the kit decides what each does to one skill.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;

/// Where a skill came from, which decides what the runtime may do with its text
/// (`SkillOriginKind` of `format/skill-types`, which is not in the core yet).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillOriginKind {
    /// A plugin's skill.
    Plugin,
    /// A skill of the managed settings directory.
    Managed,
    /// A personal skill.
    User,
    /// A project's skill.
    Project,
    /// A skill synced from claude.ai.
    Synced,
    /// A skill the host brings.
    Bundled,
}

/// The part of `SkillOrigin` the skills modules read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SkillOrigin {
    /// Where it came from.
    pub kind: SkillOriginKind,
    /// The plugin, for a plugin's skill.
    #[serde(default)]
    pub plugin: Option<String>,
}

/// The managed settings that take a skill's `allowed-tools` or `hooks` away.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpandPolicy {
    /// `strictPluginOnlyCustomization` lists `hooks`.
    #[serde(default)]
    pub hooks_locked_to_plugins: Option<bool>,
    /// `allowManagedPermissionRulesOnly` is set.
    #[serde(default)]
    pub managed_permission_rules_only: Option<bool>,
}

/// Where a plugin came from, which the host knows and the kit cannot see. Both of the last two are
/// plugins the managed settings treat like the folders they sit in:
/// - `skills-directory`: declared with a `.claude-plugin` manifest inside `~/.claude/skills/` or
///   `.claude/skills/` ("hooks from plugins other than skills-directory plugins" keep running; the
///   `allowManagedPermissionRulesOnly` entry lists them among the sources whose `allowed-tools` are
///   ignored).
/// - `inline-managed-session`: a plugin loaded for one session only (id ending in `@inline`) in a
///   Desktop Code or Cowork session whose managed settings set `allowManagedPermissionRulesOnly`, on
///   Claude Code 2.1.305 or later. The host decides all of those conditions. Any other `@inline`
///   plugin is `installed`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillPluginSource {
    /// Installed.
    #[default]
    Installed,
    /// Declared inside a skills directory.
    SkillsDirectory,
    /// Loaded for one session of a managed Desktop Code or Cowork session.
    InlineManagedSession,
}

#[derive(Clone, Copy)]
struct Keeps {
    /// The skill's frontmatter hooks register under the hooks lock.
    hooks: bool,
    /// The skill's `allowed-tools` grant applies under `allowManagedPermissionRulesOnly`.
    grant: bool,
}

const KEEPS_ALL: Keeps = Keeps {
    hooks: true,
    grant: true,
};
const KEEPS_NONE: Keeps = Keeps {
    hooks: false,
    grant: false,
};

const fn plugin_keeps(source: SkillPluginSource) -> Keeps {
    match source {
        SkillPluginSource::Installed => KEEPS_ALL,
        SkillPluginSource::SkillsDirectory => KEEPS_NONE,
        SkillPluginSource::InlineManagedSession => Keeps {
            hooks: true,
            grant: false,
        },
    }
}

/// What each origin keeps. Under the hooks lock "a skill keeps its frontmatter hooks ... only if it's
/// a plugin skill, a bundled skill, or a skill in the managed settings directory" (the kit models
/// the managed directory as the `managed` origin, which keeps both; a bundled skill is the host's, `bundled`). With only managed permission rules, "Claude Code ignores
/// `allowed-tools` in project and personal skills and in the other sources the setting's entry
/// lists", and a synced skill's grant "doesn't apply". A plugin keeps what its source keeps.
const fn keeps(kind: SkillOriginKind, source: SkillPluginSource) -> Keeps {
    match kind {
        SkillOriginKind::Plugin => plugin_keeps(source),
        SkillOriginKind::Managed => KEEPS_ALL,
        SkillOriginKind::User | SkillOriginKind::Project | SkillOriginKind::Synced => KEEPS_NONE,
        // "Skills from managed settings and bundled skills keep their `allowed-tools`"
        // (settings-reference.md, `allowManagedPermissionRulesOnly`).
        SkillOriginKind::Bundled => KEEPS_ALL,
    }
}

/// What of a skill or command the policy reads.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Source {
    /// Where it came from.
    pub origin: SkillOrigin,
    /// `fork` runs it in a subagent.
    #[serde(default)]
    pub context: Option<String>,
    /// The frontmatter `hooks`.
    #[serde(default)]
    pub hooks: Option<JsValue>,
}

/// Whether the skill's frontmatter hooks register when it is invoked.
#[must_use]
pub fn hooks_register(
    skill: &Source,
    policy: Option<ExpandPolicy>,
    source: SkillPluginSource,
) -> bool {
    policy.and_then(|policy| policy.hooks_locked_to_plugins) != Some(true)
        || keeps(skill.origin.kind, source).hooks
}

/// Whether the skill's `allowed-tools` grant applies. A skill that declares hooks under the hooks
/// lock "loses its `allowed-tools` pre-approval", except that "when such a skill runs in a subagent
/// through `context: fork`, it keeps the pre-approval". The fork exception is the hooks lock's only;
/// `allowManagedPermissionRulesOnly` ignores the field in a forked skill too.
#[must_use]
pub fn grant_applies(
    skill: &Source,
    policy: Option<ExpandPolicy>,
    source: SkillPluginSource,
) -> bool {
    if policy.and_then(|policy| policy.managed_permission_rules_only) == Some(true)
        && !keeps(skill.origin.kind, source).grant
    {
        return false;
    }
    skill.hooks.as_ref().is_none_or(JsValue::is_undefined)
        || skill.context.as_deref() == Some("fork")
        || hooks_register(skill, policy, source)
}

crate::registry::exports! {
    "skills/expand-policy" {
        "hooksRegister" => ok |skill: Source, policy: Option<ExpandPolicy>, source: Option<SkillPluginSource>| hooks_register(&skill, policy, source.unwrap_or_default());
        "grantApplies" => ok |skill: Source, policy: Option<ExpandPolicy>, source: Option<SkillPluginSource>| grant_applies(&skill, policy, source.unwrap_or_default());
    }
}
