//! Port of `src/rules/target/skill.ts`, switched.
//!
//! `Skill(name)` and `Skill(name *)` (skills.md, "Restrict Claude's skill access"): `Skill(name)`
//! is an exact match and `Skill(name *)` a prefix match with any arguments. The prefix is a whole
//! name or a namespace: `Skill(anthropic-skills *)` covers `anthropic-skills:pdf` and
//! `Skill(anthropic *)` does not, "because a prefix outside the namespace doesn't match the names
//! inside it".

use super::input::input_string;
use crate::rules::specifier_guards::is_text_specifier;
use crate::rules::types::{MatchContext, PermissionBehavior, PermissionRule, PreparedCall};

const ANY_ARGUMENTS: &str = " *";
const NAMESPACE_SEPARATOR: &str = ":";

fn skill_covers(pattern: &str, name: &str) -> bool {
    let Some(prefix) = pattern.strip_suffix(ANY_ARGUMENTS) else {
        return pattern == name;
    };
    name == prefix
        || name
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(NAMESPACE_SEPARATOR))
}

/// The names `skillNames` adds to the invoked one. A deny rule sees every other name a skill goes
/// by (its alias, its unqualified name, a synced form); an allow rule sees only "the skill's own
/// name and the name in Claude's invocation" (skills.md, "Restrict Claude's skill access"), the
/// invoked name being the call's `skill` field. The host reports the names in
/// `PreparedCall.skillNames`, the command first; the kit holds no skill catalog. Judgment
/// (spec/judgments.md): an ask rule reads the same names as an allow rule, the docs naming only
/// allow and deny.
fn names_seen(behavior: PermissionBehavior, names: &[String]) -> &[String] {
    match behavior {
        PermissionBehavior::Deny => names,
        PermissionBehavior::Allow | PermissionBehavior::Ask => names.get(..1).unwrap_or(names),
    }
}

fn names_of(behavior: PermissionBehavior, call: &PreparedCall) -> Vec<String> {
    let invoked = input_string(&call.call.input, "skill").map(str::to_owned);
    let names = call.skill_names.as_deref().unwrap_or_default();
    invoked
        .into_iter()
        .chain(names_seen(behavior, names).iter().cloned())
        .collect()
}

/// The matcher of the `skill-name` rule format.
#[must_use]
pub fn match_skill_rule(rule: &PermissionRule, call: &PreparedCall) -> bool {
    let Some(pattern) = is_text_specifier(rule.specifier.as_ref()) else {
        return false;
    };
    names_of(rule.behavior, call)
        .iter()
        .any(|name| skill_covers(pattern, name))
}

crate::registry::exports! {
    "rules/target/skill" {
        "matchSkillRule" => ok |rule: PermissionRule, call: PreparedCall, _ctx: Option<MatchContext>| match_skill_rule(&rule, &call);
    }
}
