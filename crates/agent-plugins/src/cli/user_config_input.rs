//! Port of `src/cli/user-config-input.ts`, switched.
//!
//! `userConfig` values that arrive as text from a shell (plugins/cli-reference.md, "plugin install",
//! `--config <key=value>`; "plugin configure", `--values-stdin`: "a JSON object of single-line
//! strings"): each is converted to the option's declared type and checked against it
//! (`validateUserConfigValues`: "Claude Code validates each value against the option's declared
//! type"). Pure. How a text becomes a number, a boolean or a list is a judgment (spec/judgments.md):
//! the docs show strings only.
//!
//! An option whose `type` is none of the five, which the TypeScript's table has no converter for
//! (calling it throws), keeps its text here: the check then reports the type, as for any value
//! that does not fit. A declared option is a valid manifest's, so no caller meets it.

use serde::{Deserialize, Serialize};

use crate::js::{self, JsObject, JsValue, in_property_order, property_order};
use crate::loading::resolve::user_config::ResolvedUserConfig;
use crate::loading::resolve::user_config_values::validate_user_config_values;

/// A key and the text given for it that did not make a value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigProblem {
    /// The key.
    pub key: String,
    /// The key is not among the declared options.
    pub undeclared: bool,
    /// What is wrong.
    pub message: String,
}

/// What passed and what did not.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckedInput {
    /// The values that passed, converted to their declared types.
    pub accepted: JsObject,
    /// The keys that did not.
    pub problems: Vec<ConfigProblem>,
}

type Converter = fn(&JsValue, &str) -> JsValue;

fn as_text(option: &JsValue, text: &str) -> JsValue {
    if option.get("multiple") == Some(&JsValue::Bool(true)) {
        if js::trim(text).is_empty() {
            JsValue::Array(Vec::new())
        } else {
            JsValue::Array(
                text.split(',')
                    .map(|item| JsValue::String(js::trim(item).to_owned()))
                    .collect(),
            )
        }
    } else {
        JsValue::String(text.to_owned())
    }
}

fn as_number(_option: &JsValue, text: &str) -> JsValue {
    if !js::trim(text).is_empty() && js::number(text).is_finite() {
        JsValue::Number(js::number(text))
    } else {
        JsValue::String(text.to_owned())
    }
}

fn as_boolean(_option: &JsValue, text: &str) -> JsValue {
    if text == "true" {
        return JsValue::Bool(true);
    }
    if text == "false" {
        JsValue::Bool(false)
    } else {
        JsValue::String(text.to_owned())
    }
}

/// Per declared type, how a text becomes a value; a text that does not fit stays text and fails the check.
const CONVERTERS: [(&str, Converter); 5] = [
    ("string", as_text),
    ("directory", as_text),
    ("file", as_text),
    ("number", as_number),
    ("boolean", as_boolean),
];

/// A text as the option's type: numbers and booleans are parsed, a `multiple` list splits on commas.
#[must_use]
pub fn convert_text(option: &JsValue, text: &str) -> JsValue {
    let kind = option.get("type").and_then(JsValue::as_str);
    CONVERTERS
        .iter()
        .find(|(name, _)| Some(*name) == kind)
        .map_or_else(
            || JsValue::String(text.to_owned()),
            |(_, convert)| convert(option, text),
        )
}

fn undeclared_problem(key: &str, declared: &JsObject) -> ConfigProblem {
    let known: Vec<&str> = property_order(declared)
        .into_iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let list = if known.is_empty() {
        "none".to_owned()
    } else {
        known.join(", ")
    };
    ConfigProblem {
        key: key.to_owned(),
        undeclared: true,
        message: format!(
            "\"{key}\" isn't declared in this plugin's userConfig (it declares: {list})"
        ),
    }
}

fn check_one(key: &str, text: &str, declared: &JsObject) -> Result<JsValue, ConfigProblem> {
    let Some(option) = declared.get(key).filter(|option| !option.is_undefined()) else {
        return Err(undeclared_problem(key, declared));
    };
    let value = convert_text(option, text);
    let config = ResolvedUserConfig {
        declared: JsObject::from([(key.to_owned(), option.clone())]),
        values: JsObject::from([(key.to_owned(), value.clone())]),
    };
    match validate_user_config_values(&config).into_iter().next() {
        None => Ok(value),
        Some(issue) => Err(ConfigProblem {
            key: key.to_owned(),
            undeclared: false,
            message: issue.message,
        }),
    }
}

/// Each given `key` → text against the declared options; undeclared keys and failing values are problems.
#[must_use]
pub fn check_given(declared: &JsObject, given: &JsObject) -> CheckedInput {
    let mut accepted = JsObject::new();
    let mut problems = Vec::new();
    for (key, text) in property_order(given) {
        let text = js::string_of(text);
        match check_one(key, &text, declared) {
            Ok(value) => {
                accepted.insert(key.clone(), value);
            }
            Err(problem) => problems.push(problem),
        }
    }
    CheckedInput { accepted, problems }
}

/// The options that are not set: no saved value. A `default` gives an option a value but does not set
/// it, so the dialog, `plugin configure` and the install line still count it (spec §3.4.4, §3.4.6).
#[must_use]
pub fn unset_keys(config: &ResolvedUserConfig) -> Vec<String> {
    property_order(&config.declared)
        .into_iter()
        .filter(|(key, _)| !config.values.contains_key(key.as_str()))
        .map(|(key, _)| key.clone())
        .collect()
}

/// The saved values merged with `accepted`: what a config reads once the values are stored.
#[must_use]
pub fn with_values(config: ResolvedUserConfig, accepted: JsObject) -> ResolvedUserConfig {
    let mut values = config.values;
    values.extend(accepted);
    ResolvedUserConfig {
        declared: config.declared,
        values: in_property_order(values),
    }
}

crate::registry::exports! {
    "cli/user-config-input" {
        "convertText" => ok |option: JsValue, text: String| convert_text(&option, &text);
        "checkGiven" => ok |declared: JsObject, given: JsObject| check_given(&declared, &given);
        "unsetKeys" => ok |config: ResolvedUserConfig| unset_keys(&config);
        "withValues" => ok |config: ResolvedUserConfig, accepted: JsObject| with_values(config, accepted);
    }
}
