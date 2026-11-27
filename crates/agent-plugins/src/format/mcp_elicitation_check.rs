//! Ported from `src/format/mcp-elicitation-check.ts`; only the core uses it now, so it has no wrapper.
//!
//! The check of one form value against its field (mcp.md, "Respond to MCP elicitation requests": the
//! person "fills in the fields and submits"). The docs do not say how the client checks them; the
//! constraints are those of the MCP form schema (`minimum`, `maximum`, `minLength`, `maxLength`,
//! `format`, `minItems`, `maxItems`, the listed options), and the wording is the kit's (spec/judgments.md).
//!
//! The field's shape is `ElicitationField` of `src/format/mcp-elicitation-form.ts`, whose module is
//! not in the core yet.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::url::parse;
use crate::js::{JsValue, Pattern, SPACE_CLASS, code_unit_length, number_to_string, regex_test};

/// The kind of a form field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElicitationFieldType {
    /// Text.
    #[serde(rename = "string")]
    String,
    /// A number.
    #[serde(rename = "number")]
    Number,
    /// A whole number.
    #[serde(rename = "integer")]
    Integer,
    /// A boolean.
    #[serde(rename = "boolean")]
    Boolean,
    /// One of a list of options.
    #[serde(rename = "enum")]
    Enum,
    /// Several of a list of options.
    #[serde(rename = "multi-enum")]
    MultiEnum,
}

/// A string's `format`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElicitationFormat {
    /// An email address.
    #[serde(rename = "email")]
    Email,
    /// A full URL.
    #[serde(rename = "uri")]
    Uri,
    /// A date.
    #[serde(rename = "date")]
    Date,
    /// A date and time.
    #[serde(rename = "date-time")]
    DateTime,
}

/// One field of a form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationField {
    /// The field's name.
    pub name: String,
    /// Its kind.
    #[serde(rename = "type")]
    pub field_type: ElicitationFieldType,
    /// Whether an answer is required.
    pub required: bool,
    /// Its title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Its description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The allowed values of an `enum` or `multi-enum` field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// What a dialog shows for each of `options`, in order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// The schema's `default`, when it is a value a form answer can hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<JsValue>,
    /// A string's `format`, one of `email`, `uri`, `date` and `date-time`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<ElicitationFormat>,
    /// The least a number may be.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// The most a number may be.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    /// The fewest characters of a string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<f64>,
    /// The most characters of a string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<f64>,
    /// A multi-select's lower bound on how many options are chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_items: Option<f64>,
    /// A multi-select's upper bound on how many options are chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_items: Option<f64>,
}

fn bounded(field: &ElicitationField, value: f64) -> Option<String> {
    if let Some(minimum) = field.minimum.filter(|minimum| value < *minimum) {
        return Some(format!(
            "{} must be at least {}",
            field.name,
            number_to_string(minimum)
        ));
    }
    field
        .maximum
        .filter(|maximum| value > *maximum)
        .map(|maximum| {
            format!(
                "{} must be at most {}",
                field.name,
                number_to_string(maximum)
            )
        })
}

fn one_of_message(field: &ElicitationField) -> String {
    format!(
        "{} must be one of {}",
        field.name,
        field.options.as_deref().unwrap_or_default().join(", ")
    )
}

/// `^[^\s@]+@[^\s@]+\.[^\s@]+$`, with JavaScript's `\s` (RS6).
static EMAIL: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"^[^{SPACE_CLASS}@]+@[^{SPACE_CLASS}@]+\.[^{SPACE_CLASS}@]+$"
    ))
    .ok()
});
static DATE: Pattern = LazyLock::new(|| Regex::new(r"^([0-9]{4})-([0-9]{2})-([0-9]{2})$").ok());
static DATE_TIME: Pattern = LazyLock::new(|| {
    Regex::new(
        r"^([0-9]{4}-[0-9]{2}-[0-9]{2})T([01][0-9]|2[0-3]):[0-5][0-9](:[0-5][0-9](\.[0-9]+)?)?(Z|[+-]([01][0-9]|2[0-3]):[0-5][0-9])?$",
    )
    .ok()
});

fn is_url(value: &str) -> bool {
    parse(value).is_some()
}

/// A calendar date: the month exists and the day is in it (February has 29 days in a leap year).
fn is_date(value: &str) -> bool {
    let Some(parts) = DATE.as_ref().and_then(|pattern| pattern.captures(value)) else {
        return false;
    };
    let number = |at: usize| {
        parts
            .get(at)
            .and_then(|part| part.as_str().parse::<u32>().ok())
    };
    let (Some(year), Some(month), Some(day)) = (number(1), number(2), number(3)) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn is_date_time(value: &str) -> bool {
    DATE_TIME
        .as_ref()
        .and_then(|pattern| pattern.captures(value))
        .and_then(|parts| parts.get(1))
        .is_some_and(|date| is_date(date.as_str()))
}

/// One strategy per `format` of a string: what a value must be, and how the problem reads.
fn format_of(format: ElicitationFormat) -> (fn(&str) -> bool, &'static str) {
    match format {
        ElicitationFormat::Email => (|value| regex_test(&EMAIL, value), "an email address"),
        ElicitationFormat::Uri => (is_url, "a full URL"),
        ElicitationFormat::Date => (is_date, "a date (YYYY-MM-DD)"),
        ElicitationFormat::DateTime => (is_date_time, "a date and time (ISO 8601)"),
    }
}

fn length_problem(field: &ElicitationField, value: &str) -> Option<String> {
    let length = code_unit_length(value) as f64;
    if let Some(minimum) = field.min_length.filter(|minimum| length < *minimum) {
        return Some(format!(
            "{} must have at least {} characters",
            field.name,
            number_to_string(minimum)
        ));
    }
    field
        .max_length
        .filter(|maximum| length > *maximum)
        .map(|maximum| {
            format!(
                "{} must have at most {} characters",
                field.name,
                number_to_string(maximum)
            )
        })
}

fn format_problem(field: &ElicitationField, value: &str) -> Option<String> {
    let (fits, expected) = format_of(field.format?);
    if fits(value) {
        None
    } else {
        Some(format!("{} must be {expected}", field.name))
    }
}

fn list_problem(field: &ElicitationField, value: &JsValue) -> Option<String> {
    let Some(items) = value.as_array() else {
        return Some(one_of_message(field));
    };
    let allowed = items.iter().all(|each| {
        each.as_str().is_some_and(|text| {
            field
                .options
                .as_ref()
                .is_some_and(|options| options.iter().any(|option| option == text))
        })
    });
    if !allowed {
        return Some(one_of_message(field));
    }
    let unique = items
        .iter()
        .enumerate()
        .all(|(index, each)| !items.iter().take(index).any(|before| before == each));
    if !unique {
        return Some(format!("{} must choose each option once", field.name));
    }
    let count = items.len() as f64;
    if let Some(minimum) = field.min_items.filter(|minimum| count < *minimum) {
        return Some(format!(
            "{} must choose at least {}",
            field.name,
            number_to_string(minimum)
        ));
    }
    field
        .max_items
        .filter(|maximum| count > *maximum)
        .map(|maximum| {
            format!(
                "{} must choose at most {}",
                field.name,
                number_to_string(maximum)
            )
        })
}

/// The problem `value` has as the value of `field`, or `None` when it fits.
#[must_use]
pub fn field_problem(field: &ElicitationField, value: &JsValue) -> Option<String> {
    let name = &field.name;
    match field.field_type {
        ElicitationFieldType::String => match value {
            JsValue::String(text) => {
                length_problem(field, text).or_else(|| format_problem(field, text))
            }
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::Array(_)
            | JsValue::Object(_) => Some(format!("{name} must be text")),
        },
        ElicitationFieldType::Number => match value.as_f64() {
            Some(number) if number.is_finite() => bounded(field, number),
            Some(_) | None => Some(format!("{name} must be a number")),
        },
        ElicitationFieldType::Integer => match value.as_f64() {
            Some(number) if number.is_finite() && number.fract() == 0.0 => bounded(field, number),
            Some(_) | None => Some(format!("{name} must be a whole number")),
        },
        ElicitationFieldType::Boolean => {
            if value.is_boolean() {
                None
            } else {
                Some(format!("{name} must be a boolean"))
            }
        }
        ElicitationFieldType::Enum => {
            let listed = value.as_str().is_some_and(|text| {
                field
                    .options
                    .as_ref()
                    .is_some_and(|options| options.iter().any(|option| option == text))
            });
            if listed {
                None
            } else {
                Some(one_of_message(field))
            }
        }
        ElicitationFieldType::MultiEnum => list_problem(field, value),
    }
}
