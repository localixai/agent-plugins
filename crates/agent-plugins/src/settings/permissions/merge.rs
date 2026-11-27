//! Ported from `src/settings/permissions/merge.ts`; only the core uses it now, so it has no wrapper.
//!
//! How the permission keys of several files combine. Every function takes the value of each file
//! that sets the key, highest precedence first (`KeyHolder` order), and returns one value.

use indexmap::IndexSet;

use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::settings::canonical::union_of;

fn defined(values: Vec<JsValue>) -> Vec<JsValue> {
    values
        .into_iter()
        .filter(|value| !value.is_undefined())
        .collect()
}

/// Every file's entries, none removing another's, each once, highest source first (settings.md,
/// "Lists merge instead of overriding"); `None` when no file sets the list.
fn union_or_none(lists: Vec<JsValue>) -> Option<JsValue> {
    let present = defined(lists);
    if present.is_empty() {
        return None;
    }
    let lists: Vec<Vec<JsValue>> = present
        .iter()
        .map(|list| list.as_array().cloned().unwrap_or_default())
        .collect();
    Some(JsValue::Array(union_of(&lists)))
}

fn first(values: Vec<JsValue>) -> Option<JsValue> {
    values.into_iter().find(|value| !value.is_undefined())
}

/// Fields of the block with a rule of their own; the rest are the highest file's.
const RULED: [&str; 7] = [
    "allow",
    "ask",
    "deny",
    "additionalDirectories",
    "defaultMode",
    "disableBypassPermissionsMode",
    "disableAutoMode",
];

fn field_of(blocks: &[JsValue], name: &str) -> Vec<JsValue> {
    blocks
        .iter()
        .map(|block| block.get(name).cloned().unwrap_or_default())
        .collect()
}

/// The fields the kit has no rule for here (the SDK adds some), taken from the highest file that
/// sets each. A field with a row of its own (`permissions.blockReadsOutsideWorkingDirectories`) is
/// read through that row, which holds its ladder: the block carries the highest file's value only.
fn other_fields(blocks: &[JsValue]) -> JsObject {
    let names: IndexSet<&String> = blocks
        .iter()
        .filter_map(JsValue::as_object)
        .flat_map(|block| property_order(block).into_iter().map(|(name, _)| name))
        .collect();
    names
        .into_iter()
        .filter(|name| !RULED.contains(&name.as_str()))
        .map(|name| {
            let value = first(field_of(blocks, name)).unwrap_or_default();
            (name.clone(), value)
        })
        .collect()
}

fn set(members: &mut JsObject, name: &str, value: Option<JsValue>) {
    if let Some(value) = value {
        members.insert(name.to_owned(), value);
    }
}

/// The four lists of the block: the union of every file's entries, left out when no file sets
/// one.
fn list_fields(blocks: &[JsValue], members: &mut JsObject) {
    for name in ["allow", "ask", "deny", "additionalDirectories"] {
        set(members, name, union_or_none(field_of(blocks, name)));
    }
}

/// The modes and locks: the highest file that sets each.
fn mode_fields(blocks: &[JsValue], members: &mut JsObject) {
    for name in [
        "defaultMode",
        "disableBypassPermissionsMode",
        "disableAutoMode",
    ] {
        set(members, name, first(field_of(blocks, name)));
    }
}

/// The `permissions` blocks of the files as one: `allow`, `ask`, `deny` and
/// `additionalDirectories` are the union of every file's entries; the modes and locks come from
/// the highest file that sets each, and so does every other field; `defaultMode` comes from the
/// highest file that sets it (settings.md, "Settings precedence"). Which file's `defaultMode` is
/// taken into account is decided by `startingMode`, which needs the scope this function does not
/// see.
#[must_use]
pub fn merge_permission_blocks(blocks: &[JsValue]) -> JsValue {
    let mut members = other_fields(blocks);
    list_fields(blocks, &mut members);
    mode_fields(blocks, &mut members);
    JsValue::Object(in_property_order(members))
}

/// The `autoMode` lists: each is the union of every file's entries, left out when no file sets
/// it.
fn auto_mode_lists(blocks: &[JsValue], members: &mut JsObject) {
    for name in ["environment", "allow", "soft_deny", "hard_deny"] {
        set(members, name, union_or_none(field_of(blocks, name)));
    }
}

/// The `autoMode` blocks of the files as one: "When more than one of those files sets the same
/// array, Claude Code concatenates the entries". `classifyAllShell` is the highest file's here;
/// its own row (`autoMode.classifyAllShell`) holds the ladder that decides it.
#[must_use]
pub fn merge_auto_mode_blocks(blocks: &[JsValue]) -> JsValue {
    let classify_all_shell = first(field_of(blocks, "classifyAllShell"));
    let mut members = JsObject::new();
    auto_mode_lists(blocks, &mut members);
    set(&mut members, "classifyAllShell", classify_all_shell);
    JsValue::Object(members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    #[test]
    fn lists_are_joined_and_modes_are_the_highest_file_s() {
        let blocks = json_parse(
            r#"[{"allow":["a"],"defaultMode":"plan","x":1},{"allow":["b","a"],"defaultMode":"auto"}]"#,
        )
        .unwrap_or_default();
        let merged = merge_permission_blocks(blocks.as_array().map_or(&[][..], Vec::as_slice));
        assert_eq!(
            merged.to_json_text(),
            r#"{"x":1,"allow":["a","b"],"defaultMode":"plan"}"#
        );
    }
}
