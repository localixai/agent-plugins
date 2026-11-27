//! Port of `src/format/bundled-skill.ts`, switched.
//!
//! A skill the host supplies itself: Claude Code's bundled skills ("Bundled skills": "You invoke a
//! bundled skill the same way as any other skill", skills.md). It arrives as data, not a file the kit
//! reads, so its head is read here from the name and the frontmatter the host gives.

use serde::{Deserialize, Serialize};

use super::command::command_name_issues;
use super::issues::{PluginIssue, warning};
use super::skill::directory_of;
use super::skill_fields::read_invocable;
use super::skill_name::SkillOrigin;
use crate::js::{JsObject, JsValue};

/// The head of a skill the host supplies.
#[derive(Clone, Debug, Deserialize)]
pub struct BundledSkillHead {
    /// The command, as typed without the slash: `review`, `code-review`.
    pub name: String,
    /// The frontmatter as a skill's YAML would hold it (`description`, `allowed-tools`, any key).
    #[serde(default)]
    pub frontmatter: Option<JsObject>,
    /// The skill's `SKILL.md`, absolute, when it has one: it is then a skill with a folder.
    #[serde(default)]
    pub file: Option<String>,
}

/// The skill's head, or the issues that refuse it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BundledSkillRead {
    /// The `InvocableHead`; absent when the name is refused: an error issue says why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invocable: Option<JsObject>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// `{ kind: 'bundled' }`, as the host reads it.
fn bundled() -> JsValue {
    let mut origin = JsObject::new();
    origin.insert("kind".to_owned(), "bundled".into());
    JsValue::Object(origin)
}

/// Judgment (spec/judgments.md): the name is the command as given, refused as a command name is
/// (`commandNameIssues`: an empty name, a character a command name can't hold, `anthropic-skills`),
/// and the frontmatter is read as a skill's, `name` aside (the host's `name` is the command). With a
/// `file` it is a skill whose folder is the file's directory; without one, a command without a
/// folder. `body` is the text when the host gave text, for the `description` fallback.
#[must_use]
pub fn read_bundled_skill(head: &BundledSkillHead, body: &str) -> BundledSkillRead {
    let at = format!("hostSkills.{}", head.name);
    let refused = command_name_issues(&at, std::slice::from_ref(&head.name), &SkillOrigin::Bundled);
    if !refused.is_empty() {
        return BundledSkillRead {
            invocable: None,
            issues: refused,
        };
    }
    let fields = head.frontmatter.clone().unwrap_or_default();
    let read = read_invocable(&fields, body, &["name".to_owned()]);
    let issues = read
        .problems
        .iter()
        .map(|found| warning(&at, &format!("{} {}", found.key, found.reason)))
        .collect();
    let mut invocable = read.frontmatter;
    invocable.insert("name".to_owned(), head.name.as_str().into());
    invocable.insert("command".to_owned(), head.name.as_str().into());
    invocable.insert("origin".to_owned(), bundled());
    match &head.file {
        None => {
            invocable.insert("kind".to_owned(), "command".into());
        }
        Some(file) => {
            invocable.insert("kind".to_owned(), "skill".into());
            invocable.insert("path".to_owned(), file.as_str().into());
            invocable.insert("dir".to_owned(), directory_of(file).into());
        }
    }
    BundledSkillRead {
        invocable: Some(invocable),
        issues,
    }
}

crate::registry::exports! {
    "format/bundled-skill" {
        "readBundledSkill" => ok |head: BundledSkillHead, body: String| read_bundled_skill(&head, &body);
    }
}
