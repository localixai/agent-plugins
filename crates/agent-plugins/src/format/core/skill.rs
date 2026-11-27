//! Port of `src/format/core/skill.ts`, switched.
//!
//! A core package's skill (core §7.1): its `SKILL.md` conforms to the Agent Skills specification
//! (agentskills.io/specification, "Frontmatter"), or the client skips the skill. The frontmatter is
//! required; `name` is 1–64 characters of `a-z`, `0-9` and `-`, no hyphen at either end, no `--`,
//! and the parent directory's name; `description` is 1–1024 characters; `license` is a string,
//! `compatibility` 1–500 characters, `metadata` a map of strings, `allowed-tools` a space-separated
//! string.
//!
//! Judgments (spec/judgments.md): a field the Agent Skills specification does not define is a
//! warning, not a skip, since the specification lists its fields without forbidding others (Agent
//! Plugins Extended §18.3.2 lets them exist); a description of blanks only is empty.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::format::code_points::code_point_length;
use crate::format::frontmatter::parse_frontmatter;
use crate::js::{JsValue, Pattern, property_order, regex_test, trim};
use crate::protocol::is_record::is_record;

/// What checking a skill found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreSkillCheck {
    /// Why the skill is skipped; empty when it conforms.
    pub problems: Vec<String>,
    /// Fields outside the Agent Skills specification, in the order they appear.
    pub unknown_fields: Vec<String>,
}

type Check = fn(&JsValue, &str) -> Option<String>;

fn length_between(value: &JsValue, max: usize) -> bool {
    value.as_str().is_some_and(|text| {
        let length = code_point_length(text);
        (1..=max).contains(&length)
    })
}

static NAME: Pattern = LazyLock::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").ok());

fn name_check(value: &JsValue, directory: &str) -> Option<String> {
    let Some(name) = value.as_str() else {
        return Some("\"name\" is required and must be a string".to_owned());
    };
    if !length_between(value, 64) || !regex_test(&NAME, name) {
        return Some(format!(
            "\"name\" \"{name}\" must be 1 to 64 lowercase letters, digits and single hyphens, a letter or digit at each end"
        ));
    }
    if name == directory {
        None
    } else {
        Some(format!(
            "\"name\" \"{name}\" must match its directory \"{directory}\""
        ))
    }
}

fn description_check(value: &JsValue, _directory: &str) -> Option<String> {
    if length_between(value, 1024) && value.as_str().is_some_and(|text| !trim(text).is_empty()) {
        None
    } else {
        Some("\"description\" is required: a string of 1 to 1024 characters".to_owned())
    }
}

fn optional_string(field: &str, value: &JsValue) -> Option<String> {
    if value.is_string() {
        None
    } else {
        Some(format!("\"{field}\" must be a string"))
    }
}

fn compatibility_check(value: &JsValue, _directory: &str) -> Option<String> {
    if length_between(value, 500) {
        None
    } else {
        Some("\"compatibility\" must be a string of 1 to 500 characters".to_owned())
    }
}

fn metadata_check(value: &JsValue, _directory: &str) -> Option<String> {
    let strings = match value {
        JsValue::Object(members) => members.values().all(JsValue::is_string),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Array(_) => false,
    };
    if is_record(value) && strings {
        None
    } else {
        Some("\"metadata\" must map strings to strings".to_owned())
    }
}

/// The six Agent Skills fields and their checks; `name` and `description` are checked when absent
/// too.
const FIELDS: [(&str, Check); 6] = [
    ("name", name_check),
    ("description", description_check),
    ("license", |value, _| optional_string("license", value)),
    ("compatibility", compatibility_check),
    ("metadata", metadata_check),
    ("allowed-tools", |value, _| {
        optional_string("allowed-tools", value)
    }),
];

const REQUIRED: [&str; 2] = ["name", "description"];

/// Checks a `SKILL.md` found in `skills/<directory>/` by the Agent Skills specification.
#[must_use]
pub fn check_core_skill(text: &str, directory: &str) -> CoreSkillCheck {
    let read = parse_frontmatter(text);
    if !read.present {
        return CoreSkillCheck {
            problems: vec!["SKILL.md has no frontmatter".to_owned()],
            unknown_fields: Vec::new(),
        };
    }
    if let Some(problem) = &read.problem {
        return CoreSkillCheck {
            problems: vec![format!("SKILL.md frontmatter does not parse: {problem}")],
            unknown_fields: Vec::new(),
        };
    }
    let fields = &read.fields;
    let problems = FIELDS
        .iter()
        .filter(|(field, _)| REQUIRED.contains(field) || fields.contains_key(*field))
        .filter_map(|(field, check)| {
            check(fields.get(*field).unwrap_or(&JsValue::Undefined), directory)
        })
        .collect();
    let unknown_fields = property_order(fields)
        .into_iter()
        .map(|(field, _)| field)
        .filter(|field| !FIELDS.iter().any(|(known, _)| known == field))
        .cloned()
        .collect();
    CoreSkillCheck {
        problems,
        unknown_fields,
    }
}

crate::registry::exports! {
    "format/core/skill" {
        "checkCoreSkill" => ok |text: String, directory: String| check_core_skill(&text, &directory);
    }
}
