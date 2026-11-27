//! Port of `src/format/skill.ts`, switched.
//!
//! One `SKILL.md` read into a `Skill`: its frontmatter (`readInvocable`), its name and command
//! (`resolveSkillName`), and where it is.

use serde::{Deserialize, Serialize};

use super::frontmatter::parse_frontmatter;
use super::issues::{PluginIssue, warning};
use super::skill_fields::read_invocable;
use super::skill_name::{SkillLayout, resolve_skill_name};
use crate::js::{JsObject, JsValue};

/// One skill and what was found reading it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SkillRead {
    /// The `Skill`.
    pub skill: JsObject,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// `skills/review/SKILL.md` is in `skills/review`; a file at the root is in `.`.
#[must_use]
pub fn directory_of(path: &str) -> String {
    match path.rfind(['/', '\\']) {
        None => ".".to_owned(),
        Some(cut) => path.get(..cut).unwrap_or_default().to_owned(),
    }
}

/// The command a frontmatter `name` gives a skill whose folder fixed its name, when it differs from
/// `command` (`Skill.declaredCommand`); a `name` the rules refuse gives none.
fn declared_command_of(
    layout: &SkillLayout,
    declared: Option<&str>,
    command: &str,
) -> Option<String> {
    let declared = declared.filter(|_| layout.fixed_name == Some(true))?;
    let free = SkillLayout {
        origin: layout.origin.clone(),
        dir_name: layout.dir_name.clone(),
        fixed_name: None,
    };
    let own = resolve_skill_name(&free, Some(declared));
    (own.command != command).then_some(own.command)
}

/// Read one `SKILL.md`. Never throws and never refuses the file (skills.md: "If the YAML between
/// the markers doesn't parse, the skill still loads with no fields set"; an unknown field is
/// ignored "without reporting an error"). What the docs leave open is in spec/judgments.md: a YAML
/// error and a value of the wrong shape are warnings naming the file.
///
/// The name and the command come from `resolveSkillName`; its issues are reported against the file
/// (an error means the skill is refused: a name Claude Code does not load). `origin` is the
/// layout's as the host wrote it.
#[must_use]
pub fn read_skill(path: &str, text: &str, layout: &SkillLayout, origin: JsValue) -> SkillRead {
    let parsed = parse_frontmatter(text);
    let mut issues = Vec::new();
    if let Some(problem) = &parsed.problem {
        let place = parsed
            .line
            .map_or_else(String::new, |line| format!(" (line {line})"));
        let message = format!(
            "frontmatter {problem}{place}; the skill loads with its body and no fields set"
        );
        issues.push(warning(path, &message));
    }
    let read = read_invocable(&parsed.fields, &parsed.body, &[]);
    for found in &read.problems {
        issues.push(warning(path, &format!("{} {}", found.key, found.reason)));
    }
    let declared = read.declared_name.as_ref().and_then(JsValue::as_str);
    let resolved = resolve_skill_name(layout, declared);
    issues.extend(resolved.issues.into_iter().map(|found| PluginIssue {
        severity: found.severity,
        path: path.to_owned(),
        message: found.message,
        guidance: None,
    }));
    let declared_command = declared_command_of(layout, declared, &resolved.command);
    let mut skill = read.frontmatter;
    skill.insert("kind".to_owned(), "skill".into());
    skill.insert("name".to_owned(), resolved.name.into());
    skill.insert("command".to_owned(), resolved.command.into());
    if let Some(declared_command) = declared_command {
        skill.insert("declaredCommand".to_owned(), declared_command.into());
    }
    skill.insert("path".to_owned(), path.into());
    skill.insert("dir".to_owned(), directory_of(path).into());
    skill.insert("origin".to_owned(), origin);
    skill.insert("body".to_owned(), parsed.body.into());
    SkillRead { skill, issues }
}

crate::registry::exports! {
    "format/skill" {
        "readSkill" => fallible |path: String, text: String, layout: JsValue| {
            let read = SkillLayout::deserialize(layout.clone()).map_err(|problem| problem.to_string())?;
            let origin = layout.get("origin").cloned().unwrap_or_default();
            Ok::<_, String>(read_skill(&path, &text, &read, origin))
        };
    }
}
