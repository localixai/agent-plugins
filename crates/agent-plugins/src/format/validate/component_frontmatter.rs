//! Port of `src/format/validate/component-frontmatter.ts`, switched.
//!
//! The frontmatter checks of `claude plugin validate` on a skill, agent or command file (oracle
//! 2.1.319, 2.1.321 corpus and 2.1.325; spec §4.10, §5.4.4): no block, a block that does not parse, a
//! block that is not a mapping, a missing description and the few fields whose type it reads.
//! Anything else in the block is left alone.

use serde::Deserialize;

use super::finding::{Finding, error_at, warning_at};
use super::received::kind_of;
use crate::format::frontmatter::{Frontmatter, FrontmatterState, parse_frontmatter};
use crate::js::{JsObject, JsValue};

/// The kinds of component file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
pub enum ComponentKind {
    /// A skill.
    #[serde(rename = "skill")]
    Skill,
    /// An agent.
    #[serde(rename = "agent")]
    Agent,
    /// A command.
    #[serde(rename = "command")]
    Command,
}

/// The kinds of component file.
pub const COMPONENT_KINDS: [ComponentKind; 3] = [
    ComponentKind::Skill,
    ComponentKind::Agent,
    ComponentKind::Command,
];

impl ComponentKind {
    /// The kind as its word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Agent => "agent",
            Self::Command => "command",
        }
    }
}

const EMPTY_METADATA: &str = "empty metadata (all frontmatter fields silently dropped).";

/// What a block that does not parse costs at runtime, by kind.
const fn when_unparsed(kind: ComponentKind) -> &'static str {
    match kind {
        ComponentKind::Skill | ComponentKind::Command => EMPTY_METADATA,
        ComponentKind::Agent => {
            "its name taken from the filename and every other frontmatter field silently dropped."
        }
    }
}

/// What a broken block costs an agent of a project's `.claude/agents/` (spec §2.11 table, J2.4,
/// oracle 2.1.319 and 2.1.322): it does not load, where a plugin's agent loads under its file
/// name.
const PROJECT_AGENT_UNPARSED: &str = "does not load at all \u{2014} with no frontmatter name it is treated as a co-located reference document and skipped.";

/// Where the component file sits: a plugin's folder, or a project's `.claude/` folder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum ComponentPlace {
    /// A plugin's folder.
    #[default]
    #[serde(rename = "plugin")]
    Plugin,
    /// A project's `.claude/` folder.
    #[serde(rename = "project")]
    Project,
}

/// What a broken block costs, by place and kind.
fn unparsed_cost(place: ComponentPlace, kind: ComponentKind) -> String {
    if place == ComponentPlace::Project && kind == ComponentKind::Agent {
        PROJECT_AGENT_UNPARSED.to_owned()
    } else {
        format!("loads with {}", when_unparsed(kind))
    }
}

const NO_BLOCK: &str = "No frontmatter block found. Add YAML frontmatter between --- delimiters at the top of the file to set description and other metadata.";

/// The parser's complaint, in the validator's words. The validator's YAML parser names two causes;
/// this one (`yaml`) names more, so only an unterminated quote is told apart (spec/judgments.md).
fn yaml_complaint(problem: Option<&str>) -> &'static str {
    if problem
        .unwrap_or_default()
        .to_ascii_lowercase()
        .contains("missing closing")
    {
        "Unexpected EOF"
    } else {
        "Unexpected token"
    }
}

fn sentence(value: &JsValue) -> &'static str {
    if value.is_array() {
        "an array"
    } else {
        kind_of(value)
    }
}

/// `name` and `description` name a list `object` like a map, `description` names it `array`.
fn name_kind(value: &JsValue) -> &'static str {
    if value.is_array() {
        "object"
    } else {
        kind_of(value)
    }
}

type Check = fn(&JsObject, ComponentKind) -> Vec<Finding>;

/// A key written with no value (`allowed-tools:`) is null: the validator treats it as absent.
fn present<'a>(fields: &'a JsObject, key: &str) -> Option<&'a JsValue> {
    fields
        .get(key)
        .filter(|value| !value.is_null() && !value.is_undefined())
}

fn name_check(fields: &JsObject, _kind: ComponentKind) -> Vec<Finding> {
    match present(fields, "name") {
        None => Vec::new(),
        Some(name) if name.is_string() => Vec::new(),
        Some(name) => vec![error_at(
            "name",
            &format!("name must be a string, got {}.", name_kind(name)),
        )],
    }
}

/// A list or a map; a number or a boolean is read as text and not reported.
fn description_check(fields: &JsObject, kind: ComponentKind) -> Vec<Finding> {
    let description = fields.get("description").unwrap_or(&JsValue::Undefined);
    if description.is_undefined() {
        return vec![warning_at(
            "description",
            &format!(
                "No description in frontmatter. A description helps users and Claude understand when to use this {}.",
                kind.as_str()
            ),
        )];
    }
    if description.is_object() || description.is_array() {
        vec![error_at(
            "description",
            &format!(
                "description must be a string, got {}. At runtime this value is dropped.",
                kind_of(description)
            ),
        )]
    } else {
        Vec::new()
    }
}

fn tools_check(fields: &JsObject, _kind: ComponentKind) -> Vec<Finding> {
    let Some(tools) = present(fields, "allowed-tools") else {
        return Vec::new();
    };
    if tools.is_string() {
        return Vec::new();
    }
    let Some(entries) = tools.as_array() else {
        return vec![error_at(
            "allowed-tools",
            &format!(
                "allowed-tools must be a string or array of strings, got {}.",
                kind_of(tools)
            ),
        )];
    };
    if entries.iter().all(JsValue::is_string) {
        Vec::new()
    } else {
        vec![error_at(
            "allowed-tools",
            "allowed-tools array must contain only strings.",
        )]
    }
}

const SHELLS: [&str; 2] = ["bash", "powershell"];

/// `shell`: text naming one of the two shells, in any letter case (`Bash` passes).
fn shell_check(fields: &JsObject, _kind: ComponentKind) -> Vec<Finding> {
    let Some(shell) = present(fields, "shell") else {
        return Vec::new();
    };
    let Some(text) = shell.as_str() else {
        return vec![error_at(
            "shell",
            &format!("shell must be a string, got {}.", kind_of(shell)),
        )];
    };
    if SHELLS.contains(&text.to_lowercase().as_str()) {
        Vec::new()
    } else {
        vec![error_at(
            "shell",
            &format!("shell must be 'bash' or 'powershell', got '{text}'."),
        )]
    }
}

fn metadata_check(fields: &JsObject, _kind: ComponentKind) -> Vec<Finding> {
    match present(fields, "metadata") {
        None => Vec::new(),
        Some(metadata) if metadata.is_object() => Vec::new(),
        Some(metadata) => vec![warning_at(
            "metadata",
            &format!(
                "'metadata' must be a mapping (key: value pairs); got {}. It is dropped at load time.",
                kind_of(metadata)
            ),
        )],
    }
}

/// The fields whose type the validator reads, per kind, in its order (spec §4.10 for skills and
/// commands, §5.4.4 for agents: `name` and `description` only).
fn checks(kind: ComponentKind) -> &'static [Check] {
    match kind {
        ComponentKind::Skill | ComponentKind::Command => &[
            name_check,
            tools_check,
            shell_check,
            metadata_check,
            description_check,
        ],
        ComponentKind::Agent => &[name_check, description_check],
    }
}

fn field_findings(kind: ComponentKind, fields: &JsObject) -> Vec<Finding> {
    checks(kind)
        .iter()
        .flat_map(|check| check(fields, kind))
        .collect()
}

/// What each state of the block gives: the missing block is advised on, a broken one is an error.
fn block_findings(block: &Frontmatter, kind: ComponentKind, place: ComponentPlace) -> Vec<Finding> {
    match block.state {
        FrontmatterState::None => vec![warning_at("frontmatter", NO_BLOCK)],
        FrontmatterState::Ok => field_findings(kind, &block.fields),
        FrontmatterState::InvalidYaml => vec![error_at(
            "frontmatter",
            &format!(
                "YAML frontmatter failed to parse: YAML Parse error: {}. At runtime this {} {}",
                yaml_complaint(block.problem.as_deref()),
                kind.as_str(),
                unparsed_cost(place, kind)
            ),
        )],
        FrontmatterState::NotATable => vec![error_at(
            "frontmatter",
            &format!(
                "Frontmatter must be a YAML mapping (key: value pairs), got {}.",
                sentence(block.value.as_ref().unwrap_or(&JsValue::Undefined))
            ),
        )],
    }
}

/// Findings of one component file's text, with the paths the oracle gives them.
#[must_use]
pub fn frontmatter_findings(
    kind: ComponentKind,
    text: &str,
    place: ComponentPlace,
) -> Vec<Finding> {
    let block = parse_frontmatter(text);
    block_findings(&block, kind, place)
}

crate::registry::exports! {
    "format/validate/component-frontmatter" {
        "COMPONENT_KINDS" => ok || COMPONENT_KINDS;
        "frontmatterFindings" => ok |kind: ComponentKind, text: String, place: Option<ComponentPlace>| frontmatter_findings(kind, &text, place.unwrap_or_default());
    }
}
