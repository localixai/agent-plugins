//! Port of `src/format/frontmatter-render.ts`, switched.
//!
//! A skill, command or agent file from the kit's own definition types, the inverse of `readSkill`,
//! `readCommand` and `readAgent`: each documented key in its table's order (skills.md and
//! sub-agents.md, "Frontmatter reference"), written by `writeFrontmatter`, then the host's own keys
//! (`extra`) after them. A skill or command value equal to its documented default is left out, as an
//! author leaves it out; `shell` is the exception, since an explicit `shell: bash` "fails the
//! invocation" when bash is missing (skills.md, "How injected commands run").

use serde::Deserialize;

use super::agent_fields::AGENT_FIELD_NAMES;
use super::frontmatter_write::write_frontmatter;
use super::skill_fields::{SKILL_FIELDS, invocable_defaults};
use crate::js::{JsObject, JsValue, in_property_order};

/// Each skill key's property in a draft (K1: a table over the keys).
const INVOCABLE_PROPERTY: [(&str, &str); 20] = [
    ("name", "name"),
    ("description", "description"),
    ("when_to_use", "whenToUse"),
    ("argument-hint", "argumentHint"),
    ("arguments", "arguments"),
    ("disable-model-invocation", "disableModelInvocation"),
    ("user-invocable", "userInvocable"),
    ("allowed-tools", "allowedTools"),
    ("disallowed-tools", "disallowedTools"),
    ("model", "model"),
    ("effort", "effort"),
    ("context", "context"),
    ("agent", "agent"),
    ("background", "background"),
    ("hooks", "hooks"),
    ("paths", "paths"),
    ("shell", "shell"),
    ("metadata", "metadata"),
    ("license", "license"),
    ("compatibility", "compatibility"),
];

/// Whether a value equals its documented default, `shell` aside: an empty list for a list, the
/// same primitive otherwise.
fn is_default(defaults: &JsObject, property: &str, value: &JsValue) -> bool {
    if property == "shell" {
        return false;
    }
    match defaults.get(property) {
        None => false,
        Some(JsValue::Array(_)) => matches!(value, JsValue::Array(items) if items.is_empty()),
        Some(
            fallback @ (JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::String(_)),
        ) => fallback == value,
        Some(JsValue::Object(_)) => false,
    }
}

fn invocable_fields(draft: &JsObject) -> JsObject {
    let defaults = invocable_defaults();
    let mut fields = JsObject::new();
    for key in SKILL_FIELDS {
        let property = INVOCABLE_PROPERTY
            .iter()
            .find(|(field, _)| *field == key)
            .map_or(key, |(_, property)| property);
        let Some(value) = draft.get(property).filter(|value| !value.is_undefined()) else {
            continue;
        };
        if !is_default(&defaults, property, value) {
            fields.insert(key.to_owned(), value.clone());
        }
    }
    fields
}

/// `{ ...fields, ...extra }`, in the order a JavaScript object keeps.
fn with_extra(mut fields: JsObject, extra: Option<&JsValue>) -> JsObject {
    if let Some(JsValue::Object(extra)) = extra {
        fields.extend(
            extra
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
    }
    in_property_order(fields)
}

fn text_member(draft: &JsObject, name: &str) -> String {
    draft
        .get(name)
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A skill file (a `SkillDraft`).
///
/// # Errors
///
/// What `writeFrontmatter` refuses.
pub fn render_skill_file(draft: &JsObject) -> Result<String, String> {
    let fields = with_extra(invocable_fields(draft), draft.get("extra"));
    write_frontmatter(&fields, &text_member(draft, "body"))
}

/// An agent file (`AgentDraft`): the official definition, the kit's extras and the hooks.
#[derive(Clone, Debug, Deserialize)]
pub struct AgentDraft {
    /// The agent's name.
    #[serde(default)]
    pub name: JsValue,
    /// The official shape; `prompt` is the body.
    #[serde(default)]
    pub definition: JsObject,
    /// `isolation`, `color` and `cacheTtl`.
    #[serde(default)]
    pub extras: JsValue,
    /// The agent's hooks.
    #[serde(default)]
    pub hooks: JsValue,
    /// Keys the kit does not model, written after the documented ones.
    #[serde(default)]
    pub extra: JsValue,
}

/// Each agent key's value (K1); `experimental` holds the one key the kit keeps, `cacheTtl`.
fn agent_value(draft: &AgentDraft, field: &str) -> JsValue {
    let extra = |name: &str| draft.extras.get(name).cloned().unwrap_or_default();
    match field {
        "name" => draft.name.clone(),
        "hooks" => draft.hooks.clone(),
        "isolation" | "color" => extra(field),
        "experimental" => {
            let ttl = extra("cacheTtl");
            if ttl.is_undefined() {
                JsValue::Undefined
            } else {
                JsValue::Object(JsObject::from([("cacheTtl".to_owned(), ttl)]))
            }
        }
        _ => draft.definition.get(field).cloned().unwrap_or_default(),
    }
}

/// An agent file.
///
/// # Errors
///
/// What `writeFrontmatter` refuses.
pub fn render_agent_file(draft: &AgentDraft) -> Result<String, String> {
    let fields: JsObject = AGENT_FIELD_NAMES
        .into_iter()
        .map(|key| (key.to_owned(), agent_value(draft, key)))
        .collect();
    let fields = with_extra(fields, Some(&draft.extra));
    write_frontmatter(&fields, &text_member(&draft.definition, "prompt"))
}

crate::registry::exports! {
    "format/frontmatter-render" {
        "renderSkillFile" => fallible |draft: JsObject| render_skill_file(&draft);
        "renderCommandFile" => fallible |draft: JsObject| render_skill_file(&draft);
        "renderAgentFile" => fallible |draft: AgentDraft| render_agent_file(&draft);
    }
}
