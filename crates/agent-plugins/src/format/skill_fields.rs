//! Port of `src/format/skill-fields.ts`, switched.
//!
//! The documented frontmatter fields a skill and a command share (skills.md, "Frontmatter
//! reference"), each read by its own reader, and the defaults that fill what is not written.

use indexmap::IndexMap;
use serde::Serialize;

use super::frontmatter::line_at;
use super::frontmatter_values::{
    FieldProblem, ListOptions, Separators, read_enum, read_flag, read_list, read_table, read_text,
};
use super::skill_field_readers::{
    FieldOutcome, SkillFieldValues, one, read_argument_hint, read_compatibility, read_hooks, take,
};
use super::skill_types::SKILL_SHELLS;
use crate::js::{JsObject, JsValue, code_unit_length, trim};
use crate::protocol::effort::EFFORT_LEVELS;
use crate::registry::Function;

/// The frontmatter keys of a skill, as the table of skills.md ("Frontmatter reference") lists
/// them. "A field name must match the table exactly, hyphens included: Claude Code ignores a field
/// it doesn't recognize without reporting an error", so this list is closed and an unknown key
/// (`when-to-use`, `Description`) is skipped without an issue.
pub const SKILL_FIELDS: [&str; 20] = [
    "name",
    "description",
    "when_to_use",
    "argument-hint",
    "arguments",
    "disable-model-invocation",
    "user-invocable",
    "allowed-tools",
    "disallowed-tools",
    "model",
    "effort",
    "context",
    "agent",
    "background",
    "hooks",
    "paths",
    "shell",
    "metadata",
    "license",
    "compatibility",
];

const fn words(braces: bool, separators: Separators) -> ListOptions {
    ListOptions {
        separators,
        braces: if braces { Some(true) } else { None },
    }
}

fn text_as(name: &'static str) -> impl FnOnce(String) -> SkillFieldValues {
    move |text| one(name, &text)
}

/// One reader per documented key (`FIELD_READERS`, K1: a registry typed over the keys).
#[must_use]
pub fn field_reader(field: &str, value: &JsValue, key: &str) -> FieldOutcome {
    let spaced = words(false, Separators::SpaceOrComma);
    let flag_as = |name: &'static str| move |flag: bool| one(name, &flag);
    let list_as = |name: &'static str| move |list: Vec<String>| one(name, &list);
    match field {
        "name" => take(read_text(key, value), text_as("name")),
        "description" => take(read_text(key, value), text_as("description")),
        "when_to_use" => take(read_text(key, value), text_as("whenToUse")),
        "argument-hint" => read_argument_hint(value, key),
        "arguments" => take(read_list(key, value, spaced), list_as("arguments")),
        "disable-model-invocation" => {
            take(read_flag(key, value), flag_as("disableModelInvocation"))
        }
        "user-invocable" => take(read_flag(key, value), flag_as("userInvocable")),
        "allowed-tools" => take(read_list(key, value, spaced), list_as("allowedTools")),
        "disallowed-tools" => take(read_list(key, value, spaced), list_as("disallowedTools")),
        "model" => take(read_text(key, value), text_as("model")),
        "effort" => take(read_enum(key, value, EFFORT_LEVELS), text_as("effort")),
        "context" => take(read_enum(key, value, &["fork"]), text_as("context")),
        "agent" => take(read_text(key, value), text_as("agent")),
        "background" => take(read_flag(key, value), flag_as("background")),
        "hooks" => read_hooks(value, key),
        "paths" => {
            let options = words(true, Separators::Comma);
            take(read_list(key, value, options), list_as("paths"))
        }
        "shell" => take(read_enum(key, value, SKILL_SHELLS), text_as("shell")),
        "metadata" => take(read_table(key, value), |metadata| {
            one("metadata", &JsValue::Object(metadata))
        }),
        "license" => take(read_text(key, value), text_as("license")),
        "compatibility" => read_compatibility(value, key),
        _ => FieldOutcome::default(),
    }
}

/// Run the reader of every documented key that is present; any other key is skipped.
#[must_use]
pub fn read_skill_fields(fields: &JsObject) -> FieldOutcome {
    let mut values = SkillFieldValues::new();
    let mut problems = Vec::new();
    for key in SKILL_FIELDS {
        let Some(value) = fields.get(key) else {
            continue;
        };
        let outcome = field_reader(key, value, key);
        values.extend(outcome.values);
        problems.extend(outcome.problems);
    }
    FieldOutcome { values, problems }
}

/// The first non-empty line of a Markdown body, trimmed (skills.md: `description` is optional). Read
/// line by line up to that line, so a long body is never split.
#[must_use]
pub fn first_content_line(body: &str) -> Option<String> {
    let length = code_unit_length(body);
    let mut at = 0;
    while at <= length {
        let found = line_at(body, at);
        let content = trim(&found.line);
        if !content.is_empty() {
            return Some(content.to_owned());
        }
        if found.next <= at {
            break;
        }
        at = found.next;
    }
    None
}

/// `<` and `>` in `description` and `when_to_use` "show as `&lt;` and `&gt;`" (skills.md, "Angle
/// brackets in a skill description"). The model keeps the text as written; a listing or menu that
/// shows it applies this.
#[must_use]
pub fn escape_angle_brackets(text: &str) -> String {
    text.replace('<', "&lt;").replace('>', "&gt;")
}

/// What a skill or a command reads of its frontmatter.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocableRead {
    /// The `InvocableFrontmatter`.
    pub frontmatter: JsObject,
    /// The `name` as declared, not yet resolved to a command name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_name: Option<JsValue>,
    /// What was wrong.
    pub problems: Vec<FieldProblem>,
}

/// The documented defaults of the fields a skill and a command share; the writer omits a value equal to one.
#[must_use]
pub fn invocable_defaults() -> JsObject {
    let empty = || JsValue::Array(Vec::new());
    JsObject::from([
        ("arguments".to_owned(), empty()),
        ("disableModelInvocation".to_owned(), false.into()),
        ("userInvocable".to_owned(), true.into()),
        ("allowedTools".to_owned(), empty()),
        ("disallowedTools".to_owned(), empty()),
        ("background".to_owned(), true.into()),
        ("paths".to_owned(), empty()),
        ("shell".to_owned(), "bash".into()),
    ])
}

/// The frontmatter a skill and a command share, with the documented defaults: `disable-model-
/// invocation` false, `user-invocable` true, `background` true, `shell` bash, and the first
/// non-empty body line for a missing `description`. `fields` is kept whole as `frontmatter`; the keys
/// in `untaken` are not read (a command file does not take `name` and `paths`).
#[must_use]
pub fn read_invocable(fields: &JsObject, body: &str, untaken: &[String]) -> InvocableRead {
    let taken: JsObject = fields
        .iter()
        .filter(|(key, _)| !untaken.contains(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let FieldOutcome {
        values: mut rest,
        problems,
    } = read_skill_fields(&taken);
    let name = rest.shift_remove("name");
    let present = |key: &str| rest.get(key).is_some_and(|value| !value.is_undefined());
    let description = match rest.get("description").filter(|d| !d.is_undefined()) {
        Some(written) => Some(written.clone()),
        None => first_content_line(body).map(JsValue::from),
    };
    let shell_declared = present("shell");
    let undescribed = !present("description") && !present("whenToUse");
    let mut frontmatter = invocable_defaults();
    frontmatter.extend(rest);
    if shell_declared {
        frontmatter.insert("shellDeclared".to_owned(), true.into());
    }
    if let Some(description) = description {
        frontmatter.insert("description".to_owned(), description);
    }
    if undescribed {
        frontmatter.insert("undescribed".to_owned(), true.into());
    }
    frontmatter.insert("frontmatter".to_owned(), JsValue::Object(fields.clone()));
    InvocableRead {
        frontmatter,
        declared_name: name.filter(|name| !name.is_undefined()),
        problems,
    }
}

/// One reader of `FIELD_READERS` as the host holds it: `(value, key) => FieldOutcome`.
struct Reader(&'static str);

crate::registry::methods! {
    Reader {
        "call" => ok |this, value: JsValue, key: String| field_reader(this.0, &value, &key);
    }
}

/// `FIELD_READERS` as the host holds it: a function per documented key.
fn field_readers() -> IndexMap<&'static str, Function> {
    SKILL_FIELDS
        .iter()
        .map(|field| (*field, Function::new(Reader(field))))
        .collect()
}

crate::registry::exports! {
    "format/skill-fields" {
        "SKILL_FIELDS" => ok || SKILL_FIELDS;
        "FIELD_READERS" => ok field_readers;
        "readSkillFields" => ok |fields: JsObject| read_skill_fields(&fields);
        "firstContentLine" => ok |body: String| first_content_line(&body);
        "escapeAngleBrackets" => ok |text: String| escape_angle_brackets(&text);
    }
}
