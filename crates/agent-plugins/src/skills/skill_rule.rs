//! Port of `src/skills/skill-rule.ts`, switched.
//!
//! Every name a skill goes by, for the `Skill(...)` rules that name it (skills.md, "Restrict
//! Claude's skill access", the table of what a deny rule blocks beyond the name you write). The rule
//! matcher itself is `rules/target/skill.ts`; it reads these names from `PreparedCall.skillNames`.
//! Only a deny rule sees the names beyond the invoked one ("Claude Code matches an allow rule only
//! against the skill's own name and the name in Claude's invocation"), so the host passes the list
//! to the matcher and the matcher decides who may read it.

use serde::Deserialize;

use super::expand_policy::{SkillOrigin, SkillOriginKind};

/// The namespace of skills synced from claude.ai (skills.md, "Names reserved for synced skills").
const SYNCED_NAMESPACE: &str = "anthropic-skills";
const QUALIFIER: &str = ":";

/// What of a skill or command the names read.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Named {
    /// The frontmatter name.
    pub name: String,
    /// The listed command.
    pub command: String,
    /// Where it came from.
    pub origin: SkillOrigin,
}

/// The names beyond `command` that one origin's skills go by. A registry over the origin union (K1).
fn other_names(skill: &Named) -> Vec<String> {
    match skill.origin.kind {
        // "An unqualified name: Skill(deploy) also blocks a nested skill listed as apps/web:deploy".
        // Judgment (spec/judgments.md): a plugin skill's own namespace is its plugin, so `Skill(deploy)`
        // does not reach `my-plugin:deploy`; only a nested folder's qualifier is dropped.
        SkillOriginKind::Managed | SkillOriginKind::User | SkillOriginKind::Project => {
            unqualified(skill)
        }
        SkillOriginKind::Plugin => Vec::new(),
        // A host's own skill has no qualifier; its aliases are the host's (`aliases` below).
        SkillOriginKind::Bundled => Vec::new(),
        // "A skill synced from claude.ai: Skill(anthropic-skills:deploy)" and "The plugin form of a synced
        // skill: Skill(deploy:deploy)", besides its short name.
        SkillOriginKind::Synced => vec![
            skill.name.clone(),
            format!("{SYNCED_NAMESPACE}{QUALIFIER}{}", skill.name),
            format!("{}{QUALIFIER}{}", skill.name, skill.name),
        ],
    }
}

/// The frontmatter name of a skill whose command carries a folder qualifier (`apps/web:deploy`).
fn unqualified(skill: &Named) -> Vec<String> {
    if skill
        .command
        .ends_with(&format!("{QUALIFIER}{}", skill.name))
        && skill.command != skill.name
    {
        vec![skill.name.clone()]
    } else {
        Vec::new()
    }
}

/// The skill's command first, then every other name, then the aliases the host knows (a bundled
/// skill's alias, `/review` for `/code-review`: the kit holds no catalog of bundled skills). Each
/// name once, in that order.
#[must_use]
pub fn skill_names_of(skill: &Named, aliases: &[String]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let all = std::iter::once(skill.command.clone())
        .chain(other_names(skill))
        .chain(aliases.iter().cloned());
    for name in all {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

crate::registry::exports! {
    "skills/skill-rule" {
        "skillNamesOf" => ok |skill: Named, aliases: Option<Vec<String>>| skill_names_of(&skill, &aliases.unwrap_or_default());
    }
}
