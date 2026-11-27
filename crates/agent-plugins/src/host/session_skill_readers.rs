//! Port of `src/host/session-skill-readers.ts`, switched.
//!
//! What a host reads off a `SessionSkill` row for its own list: the plugin it belongs to, the scope
//! it loaded from, and the frontmatter `keywords` some skills carry. Pure readers of the row, which
//! is data (`session-skill-catalog`'s rows hold no reader).

use serde::Serialize;

use crate::js::{self, JsValue};

/// A plugin skill's plugin: the id the host uses for it and the name its commands are scoped by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkillPlugin {
    /// The plugin's id.
    pub id: String,
    /// The plugin's name.
    pub name: String,
}

fn member<'a>(value: &'a JsValue, path: &[&str]) -> Option<&'a JsValue> {
    path.iter().try_fold(value, |at, key| at.get(key))
}

/// The plugin a skill or command row belongs to; `None` for a folder or host skill.
#[must_use]
pub fn skill_plugin(skill: &JsValue) -> Option<SkillPlugin> {
    let id = member(skill, &["source", "pluginId"])?;
    let name = member(skill, &["invocable", "origin", "plugin"])?;
    Some(SkillPlugin {
        id: js::string_of(id),
        name: js::string_of(name),
    })
}

/// Where a row loaded from: a folder scope (skills.md, "Choose where skills load"), `plugin`, or `host`.
#[must_use]
pub fn skill_scope(skill: &JsValue) -> JsValue {
    let source = skill.get("source");
    source
        .and_then(|source| source.get("scope").or_else(|| source.get("kind")))
        .cloned()
        .unwrap_or_default()
}

/// The frontmatter `keywords` of a row: a YAML list keeps its strings, a string is split at commas;
/// anything else is none. `keywords` is no field of the skills frontmatter table, and "Claude Code
/// ignores a field it doesn't recognize without reporting an error" (skills.md, "Frontmatter
/// reference"): it changes nothing the kit or Claude Code does with the skill. Judgment
/// (spec/judgments.md, `skillKeywords`): the reader is a host's own use of a key it keeps.
#[must_use]
pub fn skill_keywords(skill: &JsValue) -> Vec<String> {
    let value = member(skill, &["invocable", "frontmatter", "keywords"])
        .cloned()
        .unwrap_or_default();
    let items: Vec<JsValue> = match value {
        JsValue::String(text) => text.split(',').map(JsValue::from).collect(),
        JsValue::Array(items) => items,
        other @ (JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::Object(_)) => vec![other],
    };
    items
        .iter()
        .filter_map(JsValue::as_str)
        .map(js::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

crate::registry::exports! {
    "host/session-skill-readers" {
        "skillPlugin" => ok |skill: JsValue| skill_plugin(&skill);
        "skillScope" => ok |skill: JsValue| skill_scope(&skill);
        "skillKeywords" => ok |skill: JsValue| skill_keywords(&skill);
    }
}

#[cfg(test)]
mod tests {
    use super::{skill_keywords, skill_plugin};
    use crate::js::JsValue;

    fn parse(text: &str) -> JsValue {
        crate::js::json_parse::json_parse(text).unwrap_or_default()
    }

    #[test]
    fn keywords_split_a_string_and_keep_a_list_s_strings() {
        let row = parse(r#"{"invocable":{"frontmatter":{"keywords":" a, ,b "}}}"#);
        assert_eq!(skill_keywords(&row), ["a", "b"]);
        let row = parse(r#"{"invocable":{"frontmatter":{"keywords":["x", 1, " "]}}}"#);
        assert_eq!(skill_keywords(&row), ["x"]);
    }

    #[test]
    fn a_folder_row_has_no_plugin() {
        let row = parse(r#"{"source":{"kind":"folder","scope":"user"},"invocable":{"origin":{}}}"#);
        assert_eq!(skill_plugin(&row), None);
    }
}
