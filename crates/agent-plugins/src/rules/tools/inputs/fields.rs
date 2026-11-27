//! Port of `src/rules/tools/inputs/fields.ts` at sha256:aed2ab5cad59.
//!
//! The tables of hooks.md ("PreToolUse input"), one per tool, each field with its JSON type, its
//! role for a parameter rule, where it is documented and the zod schema of its value.
//!
//! The TypeScript's `official<I>()` ties a table to the SDK's input type `I` at compile time; the
//! core has no SDK types, so `official` only marks the source. `TOOL_INPUT_FIELDS` holds a schema
//! per field, so it never crosses the boundary: the Rust readers of tool inputs use it.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::ask_user_question::{ANSWERS_SCHEMA, QUESTIONS_SCHEMA};
use super::field_schemas::{FLAG, NUM, TEXT, choice, list};
use super::grep::GREP_OUTPUT_MODES;
use super::web_search::WEB_SEARCH_MODES;
use crate::js::value::to_js_value;
use crate::zod::{self, Schema};
use FieldRole::{Param, Primary, Subject};

/// The JSON types of the tables of hooks.md ("PreToolUse input").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    /// `string`.
    String,
    /// `number`.
    Number,
    /// `boolean`.
    Boolean,
    /// `array`.
    Array,
    /// `object`.
    Object,
}

/// What a rule can do with a field (permissions.md, "Match by input parameter"):
/// - `primary`: the content field of the tool; a parameter rule on it is ignored with a warning.
/// - `subject`: the field the tool's own specifier reads without being primary (the subagent type of
///   `Agent(Explore)`); `Agent(subagent_type:Explore)` still matches it.
/// - `param`: any other scalar; a deny or ask `Tool(field:value)` rule matches it.
/// - `structured`: an array or object; "Fields nested inside an object or array are not matchable".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldRole {
    /// `subject`.
    Subject,
    /// `param`.
    Param,
    /// `primary`.
    Primary,
    /// `structured`.
    Structured,
}

/// Where a field is documented: the SDK's input type, or only the docs (which the compiler cannot tie to the SDK).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldSource {
    /// `sdk`.
    Sdk,
    /// `docs`.
    Docs,
}

/// One field of a tool input.
#[derive(Clone, Debug)]
pub struct FieldSpec {
    /// `type`.
    pub field_type: FieldType,
    /// `optional`.
    pub optional: bool,
    /// `role`.
    pub role: FieldRole,
    /// `source`.
    pub source: FieldSource,
    /// The schema of the field's value, with `optional` already applied.
    pub schema: Schema,
}

/// `Omit<FieldSpec, 'source'>`.
#[derive(Clone, Debug)]
struct Draft {
    field_type: FieldType,
    role: FieldRole,
    optional: bool,
    schema: Schema,
}

fn draft(field_type: FieldType, role: FieldRole, schema: Schema) -> Draft {
    Draft {
        field_type,
        role,
        optional: false,
        schema,
    }
}
fn string(role: FieldRole) -> Draft {
    draft(FieldType::String, role, TEXT.clone())
}
fn number() -> Draft {
    draft(FieldType::Number, Param, NUM.clone())
}
fn boolean() -> Draft {
    draft(FieldType::Boolean, Param, FLAG.clone())
}
fn one_of(values: &[String]) -> Draft {
    draft(FieldType::String, Param, choice(values))
}
fn array(schema: Schema) -> Draft {
    draft(FieldType::Array, FieldRole::Structured, schema)
}
fn object(schema: Schema) -> Draft {
    draft(FieldType::Object, FieldRole::Structured, schema)
}
fn optional(field: Draft) -> Draft {
    Draft {
        optional: true,
        schema: field.schema.optional(),
        ..field
    }
}

/// A tool's fields, in the order its table lists them.
pub type Fields = IndexMap<String, FieldSpec>;

fn with_source<const N: usize>(source: FieldSource, drafts: [(&str, Draft); N]) -> Fields {
    drafts
        .into_iter()
        .map(|(field, entry)| {
            let spec = FieldSpec {
                field_type: entry.field_type,
                optional: entry.optional,
                role: entry.role,
                source,
                schema: entry.schema,
            };
            (field.to_owned(), spec)
        })
        .collect()
}

/// The fields of a tool that the SDK's input type has (the TypeScript checks the names against it).
fn official<const N: usize>(drafts: [(&str, Draft); N]) -> Fields {
    with_source(FieldSource::Sdk, drafts)
}

/// Documented fields the SDK's input type lacks: nothing ties them to it, so each says so.
fn documented<const N: usize>(drafts: [(&str, Draft); N]) -> Fields {
    with_source(FieldSource::Docs, drafts)
}

fn merge(parts: impl IntoIterator<Item = Fields>) -> Fields {
    parts.into_iter().flatten().collect()
}

/// The names of a list of modes, as the input spells them.
fn names<T: Serialize>(values: &[T]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| to_js_value(value).ok())
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect()
}

/// The `Bash` table; PowerShell's "fields match the Bash tool" and the SDK has no PowerShell input (spec/judgments.md).
fn shell() -> Fields {
    official([
        ("command", string(Primary)),
        ("description", optional(string(Param))),
        ("timeout", optional(number())),
        ("run_in_background", optional(boolean())),
    ])
}

/// `TOOL_INPUT_FIELDS`: one table per tool, under the tool's name.
#[derive(Clone, Debug)]
pub struct ToolInputFields {
    /// `Bash`.
    pub bash: Fields,
    /// `PowerShell`.
    pub power_shell: Fields,
    /// `Write`.
    pub write: Fields,
    /// `Edit`.
    pub edit: Fields,
    /// `Read`.
    pub read: Fields,
    /// `Glob`.
    pub glob: Fields,
    /// `Grep`.
    pub grep: Fields,
    /// `WebFetch`.
    pub web_fetch: Fields,
    /// `WebSearch`.
    pub web_search: Fields,
    /// `Agent`.
    pub agent: Fields,
    /// `AskUserQuestion`.
    pub ask_user_question: Fields,
    /// `ExitPlanMode`.
    pub exit_plan_mode: Fields,
}

impl ToolInputFields {
    /// `Object.entries(TOOL_INPUT_FIELDS)`: each tool's name and table, in the TypeScript's order.
    #[must_use]
    pub fn entries(&self) -> [(&'static str, &Fields); 12] {
        [
            ("Bash", &self.bash),
            ("PowerShell", &self.power_shell),
            ("Write", &self.write),
            ("Edit", &self.edit),
            ("Read", &self.read),
            ("Glob", &self.glob),
            ("Grep", &self.grep),
            ("WebFetch", &self.web_fetch),
            ("WebSearch", &self.web_search),
            ("Agent", &self.agent),
            ("AskUserQuestion", &self.ask_user_question),
            ("ExitPlanMode", &self.exit_plan_mode),
        ]
    }
}

/// One table per tool of hooks.md ("PreToolUse input"). Optionality is the table's ("Optional ...");
/// where it is silent (the boolean flags `run_in_background`, `replace_all`, `-i`, `multiline`, and
/// Agent, WebFetch and ExitPlanMode fields), the SDK's input type decides (spec/judgments.md).
pub static TOOL_INPUT_FIELDS: LazyLock<ToolInputFields> = LazyLock::new(|| ToolInputFields {
    bash: shell(),
    power_shell: shell(),
    write: official([("file_path", string(Primary)), ("content", string(Param))]),
    edit: official([
        ("file_path", string(Primary)),
        ("old_string", string(Param)),
        ("new_string", string(Param)),
        ("replace_all", optional(boolean())),
    ]),
    read: official([
        ("file_path", string(Primary)),
        ("offset", optional(number())),
        ("limit", optional(number())),
    ]),
    glob: official([
        ("pattern", string(Param)),
        ("path", optional(string(Primary))),
    ]),
    grep: official([
        ("pattern", string(Param)),
        ("path", optional(string(Primary))),
        ("glob", optional(string(Param))),
        ("output_mode", optional(one_of(&names(&GREP_OUTPUT_MODES)))),
        ("-i", optional(boolean())),
        ("multiline", optional(boolean())),
    ]),
    web_fetch: official([
        ("url", string(Primary)),
        ("prompt", string(Param)),
        ("offset", optional(number())),
    ]),
    web_search: merge([
        official([
            ("query", string(Param)),
            ("allowed_domains", optional(array(list(TEXT.clone())))),
            ("blocked_domains", optional(array(list(TEXT.clone())))),
        ]),
        // hooks.md, "WebSearch": "standard" or "extended"; the SDK's WebSearchInput has no `mode`.
        documented([("mode", optional(one_of(&names(&WEB_SEARCH_MODES))))]),
    ]),
    agent: official([
        ("prompt", string(Param)),
        ("description", string(Param)),
        ("subagent_type", optional(string(Subject))),
        // A string, not the SDK's four aliases: the docs say "model alias" and a full model ID can be sent.
        ("model", optional(string(Param))),
    ]),
    ask_user_question: official([
        ("questions", array(QUESTIONS_SCHEMA.clone())),
        ("answers", optional(object(ANSWERS_SCHEMA.clone()))),
    ]),
    exit_plan_mode: merge([
        // The entries of the deprecated, ignored field are not inspected.
        official([("allowedPrompts", optional(array(list(zod::unknown()))))]),
        // hooks.md, "ExitPlanMode": injected by Claude Code; the SDK's input is an open index signature.
        documented([
            ("plan", optional(string(Param))),
            ("planFilePath", optional(string(Param))),
        ]),
    ]),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_keep_the_typescript_roles_and_sources() {
        let fields = &*TOOL_INPUT_FIELDS;
        let role = |table: &Fields, field: &str| table.get(field).map(|spec| spec.role);
        assert_eq!(role(&fields.agent, "subagent_type"), Some(Subject));
        assert_eq!(role(&fields.glob, "path"), Some(Primary));
        let docs: Vec<(&str, &str)> = fields
            .entries()
            .into_iter()
            .flat_map(|(tool, table)| {
                table
                    .iter()
                    .filter(|(_, spec)| spec.source == FieldSource::Docs)
                    .map(move |(field, _)| (tool, field.as_str()))
            })
            .collect();
        assert_eq!(
            docs,
            [
                ("WebSearch", "mode"),
                ("ExitPlanMode", "plan"),
                ("ExitPlanMode", "planFilePath")
            ]
        );
        let modes = fields
            .grep
            .get("output_mode")
            .map(|spec| spec.schema.clone());
        let refused = modes.and_then(|schema| schema.safe_parse(&"lines".into()).err());
        let message = refused.and_then(|failure| {
            failure
                .issues
                .first()
                .map(|found| found.message().to_owned())
        });
        assert_eq!(
            message.as_deref(),
            Some("expected one of content, files_with_matches, count")
        );
    }
}
