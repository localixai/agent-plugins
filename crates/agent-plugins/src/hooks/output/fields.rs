//! Ported from `src/hooks/output/fields.ts`; only the core uses it now, so it has no wrapper.
//!
//! Reading a validated `hookSpecificOutput` without a cast: the schema already checked every field,
//! so these only look a field up and narrow it.

use crate::js::JsValue;

fn value_in<'a>(source: Option<&'a JsValue>, key: &str) -> Option<&'a JsValue> {
    source?.as_object()?.get(key)
}

/// A field that is a string.
#[must_use]
pub fn string_in(source: Option<&JsValue>, key: &str) -> Option<String> {
    value_in(source, key)
        .and_then(JsValue::as_str)
        .map(str::to_owned)
}

/// A field that is a boolean.
#[must_use]
pub fn bool_in(source: Option<&JsValue>, key: &str) -> Option<bool> {
    value_in(source, key).and_then(JsValue::as_bool)
}

/// A string with something in it: an empty `additionalContext` adds nothing.
#[must_use]
pub fn has_text(text: Option<&str>) -> bool {
    text.is_some_and(|text| !text.is_empty())
}

/// A non-empty string of the field.
#[must_use]
pub fn text_in(source: Option<&JsValue>, key: &str) -> Option<String> {
    string_in(source, key).filter(|value| !value.is_empty())
}

/// A field that is an array of strings.
#[must_use]
pub fn strings_in(source: Option<&JsValue>, key: &str) -> Option<Vec<String>> {
    value_in(source, key)?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}

/// `source` as the official `hookSpecificOutput` of `event`, when it is one.
#[must_use]
pub fn specific_of<'a>(event: &str, source: Option<&'a JsValue>) -> Option<&'a JsValue> {
    let named = value_in(source, "hookEventName")?.as_str()?;
    if named == event { source } else { None }
}
