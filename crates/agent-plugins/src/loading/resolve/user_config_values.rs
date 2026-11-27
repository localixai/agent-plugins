//! Port of `src/loading/resolve/user-config-values.ts`, switched.
//!
//! The checks of a saved `userConfig` value that the manifest's fields state (plugins/
//! manifest-reference.md, "User configuration"): `type` ("One of `string`, `number`, `boolean`,
//! `directory`, or `file`"), `required` ("the configuration dialog doesn't accept an empty value"),
//! `multiple` ("allows an array of strings"), `options` ("the values the field accepts") and `min` /
//! `max` ("Bounds for `number`"). The dialog is the host's; this tells it, and a loader that reads
//! saved settings, whether a value would be accepted. Judgment: the value checked is the one
//! `${user_config.KEY}` stands for, the saved one else the `default`, and a value that is not
//! declared is not checked (spec/judgments.md).

use super::user_config::{ResolvedUserConfig, user_config_value};
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsValue, array_join, string_of};

const TEXT_TYPES: [&str; 3] = ["string", "directory", "file"];

fn is_empty(value: Option<&JsValue>) -> bool {
    match value {
        None | Some(JsValue::Undefined) => true,
        Some(JsValue::String(text)) => text.is_empty(),
        Some(JsValue::Array(items)) => items.is_empty(),
        Some(JsValue::Null | JsValue::Bool(_) | JsValue::Number(_) | JsValue::Object(_)) => false,
    }
}

fn member<'a>(option: &'a JsValue, name: &str) -> Option<&'a JsValue> {
    option.get(name).filter(|value| !value.is_undefined())
}

fn type_problem(option: &JsValue, value: &JsValue) -> Option<String> {
    let kind = member(option, "type").map_or_else(|| "undefined".to_owned(), string_of);
    let shown = value.type_of();
    if TEXT_TYPES.contains(&kind.as_str()) {
        if member(option, "multiple") == Some(&JsValue::Bool(true)) {
            let list = value
                .as_array()
                .is_some_and(|items| items.iter().all(|each| each.as_str().is_some()));
            return (!list).then(|| format!("expects an array of strings (multiple), not {shown}"));
        }
        return value
            .as_str()
            .is_none()
            .then(|| format!("expects a {kind}, not {shown}"));
    }
    let finite = match value {
        JsValue::Number(number) => number.is_finite(),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Bool(_)
        | JsValue::String(_)
        | JsValue::Array(_)
        | JsValue::Object(_) => true,
    };
    if shown == kind && (kind != "number" || finite) {
        return None;
    }
    let named = if value.as_array().is_some() {
        "array"
    } else {
        shown
    };
    Some(format!("expects a {kind}, not {named}"))
}

fn limit_problem(option: &JsValue, value: &JsValue) -> Option<String> {
    if let JsValue::Number(number) = value {
        if let Some(JsValue::Number(min)) = member(option, "min")
            && number < min
        {
            return Some(format!(
                "is below the minimum {}",
                string_of(&JsValue::Number(*min))
            ));
        }
        if let Some(JsValue::Number(max)) = member(option, "max")
            && number > max
        {
            return Some(format!(
                "is above the maximum {}",
                string_of(&JsValue::Number(*max))
            ));
        }
    }
    let options = member(option, "options").and_then(JsValue::as_array)?;
    let text = value.as_str()?;
    if options.iter().any(|each| each.as_str() == Some(text)) {
        return None;
    }
    Some(format!(
        "is not one of the options: {}",
        array_join(options, ", ")
    ))
}

fn problem_of(option: &JsValue, value: Option<&JsValue>) -> Option<String> {
    if is_empty(value) {
        let required = member(option, "required") == Some(&JsValue::Bool(true));
        return required.then(|| "is required and cannot be empty".to_owned());
    }
    let value = value?;
    type_problem(option, value).or_else(|| limit_problem(option, value))
}

/// One error per declared option whose value would not be accepted: `userConfig.<key>`.
#[must_use]
pub fn validate_user_config_values(config: &ResolvedUserConfig) -> Vec<PluginIssue> {
    config
        .declared
        .iter()
        .filter_map(|(key, option)| {
            let value = user_config_value(config, key);
            let problem = problem_of(option, value.as_ref())?;
            let title = member(option, "title").map_or_else(|| "undefined".to_owned(), string_of);
            Some(error(
                &format!("userConfig.{key}"),
                &format!("\"{title}\" {problem}"),
            ))
        })
        .collect()
}

crate::registry::exports! {
    "loading/resolve/user-config-values" {
        "validateUserConfigValues" => ok |config: ResolvedUserConfig| validate_user_config_values(&config);
    }
}
