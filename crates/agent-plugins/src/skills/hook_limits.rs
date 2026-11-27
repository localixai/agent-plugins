//! Port of `src/skills/hook-limits.ts`, switched.
//!
//! The limits on the hooks a skill registers (hooks.md, "Limits on skill hooks"): per source of
//! skills and per session, at most 1,024 hooks and 16,384 characters of matchers, built up skill by
//! skill. Pure counting: whether a session keeps the totals, and registering and firing the hooks,
//! are the host's and the hooks engine's.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::expand_policy::{SkillOrigin, SkillOriginKind};
use crate::js::JsValue;
use crate::js::indexing::length_number;

/// The limits.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillHookLimits {
    /// At most this many hooks.
    pub hooks: f64,
    /// At most this many characters of matchers.
    pub matcher_chars: f64,
}

/// At most 1,024 hooks and 16,384 characters of matchers.
pub const SKILL_HOOK_LIMITS: SkillHookLimits = SkillHookLimits {
    hooks: 1024.0,
    matcher_chars: 16_384.0,
};

/// What a source has registered.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookUsage {
    /// Hooks.
    pub hooks: f64,
    /// Characters of matchers.
    pub matcher_chars: f64,
}

/// Nothing registered yet.
pub const NO_HOOKS_YET: HookUsage = HookUsage {
    hooks: 0.0,
    matcher_chars: 0.0,
};

/// A matcher group of a skill's `hooks` (`HookMatcherGroup`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct MatcherGroup {
    /// The matcher, when there is one.
    #[serde(default)]
    pub matcher: Option<String>,
    /// The hooks under it.
    pub hooks: Vec<JsValue>,
}

/// A skill's `hooks`: the groups of each event.
pub type HooksSettings = IndexMap<String, Vec<MatcherGroup>>;

/// The hooks and matcher characters of one skill's `hooks`: "A matcher with several hooks under it
/// counts once for each hook". Judgment (spec/judgments.md): characters are code points, and a group
/// with no matcher adds none.
#[must_use]
pub fn count_skill_hooks(hooks: &HooksSettings) -> HookUsage {
    let mut count = 0.0;
    let mut chars = 0.0;
    for groups in hooks.values() {
        for group in groups {
            let hooks = length_number(group.hooks.len());
            count += hooks;
            chars += length_number(group.matcher.as_deref().unwrap_or("").chars().count()) * hooks;
        }
    }
    HookUsage {
        hooks: count,
        matcher_chars: chars,
    }
}

/// Whether the skill's hooks were admitted, and the source's totals after it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct HooksAdmission {
    /// The skill's hooks fit.
    pub admitted: bool,
    /// The source's totals after this skill: unchanged when it was refused.
    pub used: HookUsage,
}

/// Whether a skill's hooks fit what its source already registered. A skill that does not fit
/// "registers none of its hooks", so the totals do not move and a later, smaller skill may still fit.
#[must_use]
pub fn admit_skill_hooks(used: HookUsage, hooks: &HooksSettings) -> HooksAdmission {
    let added = count_skill_hooks(hooks);
    let next = HookUsage {
        hooks: used.hooks + added.hooks,
        matcher_chars: used.matcher_chars + added.matcher_chars,
    };
    let fits = next.hooks <= SKILL_HOOK_LIMITS.hooks
        && next.matcher_chars <= SKILL_HOOK_LIMITS.matcher_chars;
    if fits {
        HooksAdmission {
            admitted: true,
            used: next,
        }
    } else {
        HooksAdmission {
            admitted: false,
            used,
        }
    }
}

/// The source a skill counts under: the project, the personal folder, or one plugin. Judgment: the
/// docs name no limit for a skill synced from claude.ai or a bundled skill, so each counts as a
/// source of its own.
#[must_use]
pub fn hook_source_of(origin: &SkillOrigin) -> String {
    match origin.kind {
        SkillOriginKind::Plugin => format!("plugin:{}", origin.plugin.as_deref().unwrap_or("")),
        SkillOriginKind::Managed => "managed".to_owned(),
        SkillOriginKind::User => "user".to_owned(),
        SkillOriginKind::Project => "project".to_owned(),
        SkillOriginKind::Synced => "synced".to_owned(),
        SkillOriginKind::Bundled => "bundled".to_owned(),
    }
}

crate::registry::exports! {
    "skills/hook-limits" {
        "SKILL_HOOK_LIMITS" => ok || SKILL_HOOK_LIMITS;
        "NO_HOOKS_YET" => ok || NO_HOOKS_YET;
        "countSkillHooks" => ok |hooks: HooksSettings| count_skill_hooks(&hooks);
        "admitSkillHooks" => ok |used: HookUsage, hooks: HooksSettings| admit_skill_hooks(used, &hooks);
        "hookSourceOf" => ok |origin: SkillOrigin| hook_source_of(&origin);
    }
}
