//! Port of `src/format/mcp-elicitation-form.ts`, switched.
//!
//! The fields of a form-mode elicitation request and the check of an answer against them (mcp.md,
//! "Respond to MCP elicitation requests": "Claude Code shows a dialog with form fields defined by
//! the server"). Pure data: the dialog is the host's.
//!
//! The field's own types (`ElicitationField`, `ElicitationFieldType`, `ElicitationFormat`) are
//! declared with the check that reads them, `mcp_elicitation_check`.

use indexmap::IndexMap;
use serde::Serialize;

use super::mcp_elicitation_check::field_problem;
pub use super::mcp_elicitation_check::{ElicitationField, ElicitationFieldType, ElicitationFormat};
use crate::js::{JsObject, JsValue, in_key_order, property_order};

/// What MCP lets a form answer hold: text, a number, a boolean or a list of text (`isValue`).
pub type ElicitationValue = JsValue;

/// The `format`s a string may name.
pub const ELICITATION_FORMATS: [ElicitationFormat; 4] = [
    ElicitationFormat::Email,
    ElicitationFormat::Uri,
    ElicitationFormat::Date,
    ElicitationFormat::DateTime,
];

const fn format_name(format: ElicitationFormat) -> &'static str {
    match format {
        ElicitationFormat::Email => "email",
        ElicitationFormat::Uri => "uri",
        ElicitationFormat::Date => "date",
        ElicitationFormat::DateTime => "date-time",
    }
}

fn strings(value: &JsValue) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|each| each.as_str().map(str::to_owned))
        .collect()
}

struct Choices {
    options: Vec<String>,
    labels: Vec<String>,
}

fn member<'a>(shape: &'a JsObject, key: &str) -> &'a JsValue {
    shape.get(key).unwrap_or(&JsValue::Undefined)
}

/// `oneOf` or `anyOf` of `{ const, title }` entries: the values, labelled by their titles.
fn const_choices(schema: &JsObject) -> Option<Choices> {
    let alternatives = [member(schema, "oneOf"), member(schema, "anyOf")]
        .into_iter()
        .find_map(JsValue::as_array)?;
    let entries: Vec<&JsObject> = alternatives
        .iter()
        .filter_map(JsValue::as_object)
        .filter(|each| member(each, "const").is_string())
        .collect();
    if entries.len() != alternatives.len() {
        return None;
    }
    let text = |value: &JsValue| value.as_str().unwrap_or_default().to_owned();
    let options = entries
        .iter()
        .map(|entry| text(member(entry, "const")))
        .collect();
    let labels = entries
        .iter()
        .map(|entry| {
            let title = member(entry, "title");
            if title.is_string() {
                text(title)
            } else {
                text(member(entry, "const"))
            }
        })
        .collect();
    Some(Choices { options, labels })
}

/// `enum: [...]` (labelled by a matching `enumNames`) or `oneOf: [{ const }]`, the ways the MCP
/// schema lists the values of a choice.
fn choices_of(schema: &JsObject) -> Option<Choices> {
    let Some(listed) = strings(member(schema, "enum")) else {
        return const_choices(schema);
    };
    let labels = strings(member(schema, "enumNames"))
        .filter(|names| names.len() == listed.len())
        .unwrap_or_else(|| listed.clone());
    Some(Choices {
        options: listed,
        labels,
    })
}

const SCALARS: [(&str, ElicitationFieldType); 4] = [
    ("string", ElicitationFieldType::String),
    ("number", ElicitationFieldType::Number),
    ("integer", ElicitationFieldType::Integer),
    ("boolean", ElicitationFieldType::Boolean),
];

/// Judgment (spec/judgments.md): a property with no type the kit knows is a text field.
fn type_of(
    shape: &JsObject,
    items: Option<&Choices>,
    own: Option<&Choices>,
) -> ElicitationFieldType {
    let declared = member(shape, "type");
    if declared.as_str() == Some("array") && items.is_some() {
        return ElicitationFieldType::MultiEnum;
    }
    if own.is_some() {
        return ElicitationFieldType::Enum;
    }
    declared
        .as_str()
        .and_then(|name| SCALARS.iter().find(|(known, _)| *known == name))
        .map_or(ElicitationFieldType::String, |(_, named)| *named)
}

fn number_of(shape: &JsObject, key: &str) -> Option<f64> {
    member(shape, key).as_f64()
}

fn text_of(shape: &JsObject, key: &str) -> Option<String> {
    member(shape, key).as_str().map(str::to_owned)
}

/// What MCP lets a form answer hold: text, a number, a boolean or a list of text.
fn is_value(value: &JsValue) -> bool {
    value.is_string() || value.is_number() || value.is_boolean() || strings(value).is_some()
}

/// What a form answer can hold: the schema's `default` when it is such a value.
fn default_of(shape: &JsObject) -> Option<JsValue> {
    Some(member(shape, "default"))
        .filter(|value| is_value(value))
        .cloned()
}

fn format_of(shape: &JsObject) -> Option<ElicitationFormat> {
    let named = member(shape, "format").as_str()?;
    ELICITATION_FORMATS
        .into_iter()
        .find(|format| format_name(*format) == named)
}

fn field_of(name: &str, schema: &JsValue, required: bool) -> ElicitationField {
    let empty = JsObject::new();
    let shape = schema.as_object().unwrap_or(&empty);
    let items = member(shape, "items").as_object().and_then(choices_of);
    let own = choices_of(shape);
    let field_type = type_of(shape, items.as_ref(), own.as_ref());
    let choices = match field_type {
        ElicitationFieldType::MultiEnum => items,
        ElicitationFieldType::Enum => own,
        ElicitationFieldType::String
        | ElicitationFieldType::Number
        | ElicitationFieldType::Integer
        | ElicitationFieldType::Boolean => None,
    };
    ElicitationField {
        name: name.to_owned(),
        field_type,
        required,
        title: text_of(shape, "title"),
        description: text_of(shape, "description"),
        options: choices.as_ref().map(|found| found.options.clone()),
        labels: choices.map(|found| found.labels),
        default: default_of(shape),
        format: format_of(shape),
        minimum: number_of(shape, "minimum"),
        maximum: number_of(shape, "maximum"),
        min_length: number_of(shape, "minLength"),
        max_length: number_of(shape, "maxLength"),
        min_items: number_of(shape, "minItems"),
        max_items: number_of(shape, "maxItems"),
    }
}

/// The fields of a form request: its `requestedSchema` properties, in order, with `required`
/// marked.
#[must_use]
pub fn elicitation_form_fields(schema: Option<&JsObject>) -> Vec<ElicitationField> {
    let Some(JsValue::Object(properties)) = schema.and_then(|schema| schema.get("properties"))
    else {
        return Vec::new();
    };
    let required = schema
        .and_then(|schema| strings(member(schema, "required")))
        .unwrap_or_default();
    property_order(properties)
        .into_iter()
        .map(|(name, each)| field_of(name, each, required.contains(name)))
        .collect()
}

/// The values of an answer a form can hold, by field.
#[must_use]
pub fn content_of(values: &JsObject) -> IndexMap<String, ElicitationValue> {
    in_key_order(
        property_order(values)
            .into_iter()
            .filter(|(_, value)| is_value(value))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
    )
}

/// The problem each field's value has, by field name; empty when the values fit. A field with no
/// value is a problem only when it is required. Judgment (spec/judgments.md): the wording is the
/// kit's.
#[must_use]
pub fn elicitation_field_problems(
    schema: Option<&JsObject>,
    values: &JsObject,
) -> IndexMap<String, String> {
    let problems = elicitation_form_fields(schema)
        .into_iter()
        .filter_map(|field| {
            let value = member(values, &field.name);
            if value.is_undefined() {
                return field
                    .required
                    .then(|| (field.name.clone(), format!("{} is required", field.name)));
            }
            field_problem(&field, value).map(|problem| (field.name.clone(), problem))
        })
        .collect();
    in_key_order(problems)
}

/// What accepting a form sends.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AcceptedForm {
    /// `accept`.
    pub action: &'static str,
    /// The values.
    pub content: IndexMap<String, ElicitationValue>,
}

/// The answer to a form: the result to send, or what is wrong with the values.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ElicitationAnswer {
    /// The result to send.
    Result {
        /// The accepted form.
        result: AcceptedForm,
    },
    /// What is wrong with the values.
    Problems {
        /// One text per field with a problem.
        problems: Vec<String>,
    },
}

/// The answer to a form request (`request.requestedSchema` is the one field of the request read).
#[must_use]
pub fn form_answer(request: &JsValue, values: &JsObject) -> ElicitationAnswer {
    let schema = request.get("requestedSchema").and_then(JsValue::as_object);
    let problems: Vec<String> = elicitation_field_problems(schema, values)
        .into_values()
        .collect();
    if !problems.is_empty() {
        return ElicitationAnswer::Problems { problems };
    }
    let known: Vec<String> = elicitation_form_fields(schema)
        .into_iter()
        .map(|field| field.name)
        .collect();
    let sent: JsObject = in_key_order(
        property_order(values)
            .into_iter()
            .filter(|(name, _)| known.contains(name))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
    );
    ElicitationAnswer::Result {
        result: AcceptedForm {
            action: "accept",
            content: content_of(&sent),
        },
    }
}

crate::registry::exports! {
    "format/mcp-elicitation-form" {
        "ELICITATION_FORMATS" => ok || ELICITATION_FORMATS;
        "elicitationFormFields" => ok |schema: Option<JsObject>| elicitation_form_fields(schema.as_ref());
        "contentOf" => ok |values: JsObject| content_of(&values);
        "elicitationFieldProblems" => ok |schema: Option<JsObject>, values: JsObject| elicitation_field_problems(schema.as_ref(), &values);
        "formAnswer" => ok |request: JsValue, values: JsObject| form_answer(&request, &values);
    }
}
