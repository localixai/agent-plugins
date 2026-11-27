//! Port of `src/settings/paths.ts`, switched.

use serde::Deserialize;

use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::protocol::is_record::is_record;

/// A key inside a block of a settings object, the way the docs name it: dotted
/// (`sandbox.network.allowedDomains`) or as its names down the blocks.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum SettingsPath {
    /// The dotted name.
    Dotted(String),
    /// The names down the blocks.
    Names(Vec<String>),
}

fn names_of(path: &SettingsPath) -> Vec<&str> {
    match path {
        SettingsPath::Dotted(text) => text.split('.').collect(),
        SettingsPath::Names(names) => names.iter().map(String::as_str).collect(),
    }
}

/// The value at `path`, whatever the blocks on the way; `undefined` when a block is missing.
#[must_use]
pub fn value_at(settings: &JsValue, path: &SettingsPath) -> JsValue {
    names_of(path)
        .into_iter()
        .try_fold(settings, |block, name| {
            if is_record(block) {
                block.get(name)
            } else {
                None
            }
        })
        .cloned()
        .unwrap_or_default()
}

fn is_empty_block(value: &JsValue) -> bool {
    value.as_object().is_some_and(JsObject::is_empty)
}

/// `{ ...block }`: a copy, its members in the order `Object.keys` lists them.
fn copy_of(block: &JsObject) -> JsObject {
    in_property_order(block.clone())
}

fn written(block: &JsObject, names: &[&str], value: &JsValue) -> JsObject {
    let Some((name, rest)) = names.split_first() else {
        return copy_of(block);
    };
    let inner = block.get(*name);
    // Nothing to remove where the block is not there: no empty block is made for it.
    if value.is_undefined() && !rest.is_empty() && !inner.is_some_and(is_record) {
        return copy_of(block);
    }
    let next = if rest.is_empty() {
        value.clone()
    } else {
        let empty = JsObject::new();
        let below = inner
            .filter(|inner| is_record(inner))
            .and_then(JsValue::as_object)
            .unwrap_or(&empty);
        JsValue::Object(written(below, rest, value))
    };
    // A block the removal emptied is no block (a project file's `autoMode` held that one key).
    let absent = next.is_undefined() || (value.is_undefined() && is_empty_block(&next));
    let mut entries: JsObject = property_order(block)
        .into_iter()
        .filter(|(existing, _)| existing.as_str() != *name)
        .map(|(existing, member)| (existing.clone(), member.clone()))
        .collect();
    if !absent {
        entries.insert((*name).to_owned(), next);
    }
    in_property_order(entries)
}

/// The settings with `value` at `path`, the blocks on the way made; with `value` `undefined` the key
/// is removed, and so is a block the removal leaves empty.
#[must_use]
pub fn with_value_at(settings: &JsValue, path: &SettingsPath, value: &JsValue) -> JsValue {
    let empty = JsObject::new();
    JsValue::Object(written(
        settings.as_object().unwrap_or(&empty),
        &names_of(path),
        value,
    ))
}

crate::registry::exports! {
    "settings/paths" {
        "valueAt" => nullable |settings: JsValue, path: SettingsPath| value_at(&settings, &path);
        "withValueAt" => ok |settings: JsValue, path: SettingsPath, value: JsValue| with_value_at(&settings, &path, &value);
    }
}
