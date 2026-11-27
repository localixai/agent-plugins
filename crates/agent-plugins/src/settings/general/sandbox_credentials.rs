//! Ported from `src/settings/general/sandbox-credentials.ts`; only the core uses it now, so it has no
//! wrapper.
//!
//! Managed `sandbox.credentials` entries (settings-reference.md, "Invalid credential entries in
//! managed settings"): an entry in `files` or `envVars` with a valid `path` or `name` and a `mode`
//! of `mask` or `deny` that fails validation anywhere else "is degraded to `mode: "deny"` with a
//! warning, so the credential stays blocked, not masked"; an entry with an unknown `mode` or an
//! invalid `path` or `name` is stripped. Each case warns and the valid entries stay. A `deny` entry
//! accepts the mask fields and ignores them, so only a `mask` entry is checked against them. The
//! two lists have their own rules (the tables "Mask fields for files" and "Mask fields for
//! environment variables").

use serde::Serialize;

use crate::format::issues::PluginIssue;
use crate::js::regexp::RegExp;
use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::protocol::is_record::is_record;
use crate::settings::repair::support::{RepairAt, note};

const MODES: [&str; 2] = ["deny", "mask"];
const DENY_MODES: [&str; 1] = ["deny"];

fn mode_in(entry: &JsValue, modes: &[&str]) -> bool {
    entry
        .get("mode")
        .and_then(JsValue::as_str)
        .is_some_and(|mode| modes.contains(&mode))
}

/// Whether a credential entry has `mode: "deny"`: it blocks the credential and needs no repair.
#[must_use]
pub fn is_deny_entry(entry: &JsValue) -> bool {
    is_record(entry) && mode_in(entry, &DENY_MODES)
}

fn is_texts(value: &JsValue) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(JsValue::is_string))
}

/// "at least one claim name".
fn is_claims(value: &JsValue) -> bool {
    is_texts(value) && value.as_array().is_some_and(|items| !items.is_empty())
}

/// "A regular expression with at least one capturing group".
fn is_extract(value: &JsValue) -> bool {
    let Some(text) = value.as_str() else {
        return false;
    };
    // The pattern must compile and hold a capturing group: `|` adds an empty alternative, so it
    // matches.
    RegExp::new(&format!("{text}|"), "")
        .ok()
        .and_then(|mut pattern| pattern.exec(""))
        .is_some_and(|found| found.captures.len() > 1)
}

/// The mask fields both lists take; `maskDuplicates` is the files list's own.
fn field_valid(field: &str, value: &JsValue, with_duplicates: bool) -> bool {
    match field {
        "extract" => is_extract(value),
        "onExtractNoMatch" => matches!(value.as_str(), Some("warn" | "deny" | "error")),
        "decode" => value.as_str() == Some("jwt"),
        "maskClaims" => is_claims(value),
        "injectHosts" => is_texts(value),
        "maskDuplicates" if with_duplicates => value.is_boolean(),
        _ => false,
    }
}

fn unset(entry: &JsValue, field: &str) -> bool {
    entry.get(field).is_none_or(JsValue::is_undefined)
}

/// `maskClaims` "requires `decode`".
fn claims_need_decode(entry: &JsValue) -> bool {
    unset(entry, "maskClaims") || !unset(entry, "decode")
}

/// The two lists.
#[derive(Clone, Copy)]
enum Group {
    Files,
    EnvVars,
}

const GROUPS: [Group; 2] = [Group::Files, Group::EnvVars];

/// An environment variable "must start with a letter or underscore and contain only letters,
/// digits, and underscores": `/^[A-Za-z_][A-Za-z0-9_]*$/`.
fn is_env_name(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
}

impl Group {
    const fn list(self) -> &'static str {
        match self {
            Self::Files => "files",
            Self::EnvVars => "envVars",
        }
    }

    const fn identity(self) -> &'static str {
        match self {
            Self::Files => "path",
            Self::EnvVars => "name",
        }
    }

    fn identity_valid(self, value: Option<&JsValue>) -> bool {
        let text = value.and_then(JsValue::as_str);
        match self {
            Self::Files => text.is_some_and(|text| !text.is_empty()),
            Self::EnvVars => text.is_some_and(is_env_name),
        }
    }

    /// What an entry's fields must satisfy together.
    fn rules(self, entry: &JsValue) -> bool {
        match self {
            Self::Files => claims_need_decode(entry),
            Self::EnvVars => {
                claims_need_decode(entry)
                    // "`extract` and `decode` can't be combined on the same entry."
                    && (unset(entry, "extract") || unset(entry, "decode"))
                    // "On an entry with `decode`, only `"warn"` is accepted."
                    && (unset(entry, "decode")
                        || entry
                            .get("onExtractNoMatch")
                            .filter(|value| !value.is_nullish())
                            .is_none_or(|value| value.as_str() == Some("warn")))
            }
        }
    }

    /// What the warning of a degraded entry adds.
    const fn degraded_note(self) -> &'static str {
        match self {
            // "A degraded `files` entry pins `filesystem.disabled` like an explicit `deny` entry".
            Self::Files => {
                ". Its read block is not enforced if managed settings turn filesystem isolation off"
            }
            Self::EnvVars => "",
        }
    }
}

/// Whether every field the entry sets beyond its identity and mode is valid for the list, and
/// together.
fn is_whole(entry: &JsObject, value: &JsValue, group: Group) -> bool {
    property_order(entry).into_iter().all(|(field, member)| {
        field == group.identity()
            || field == "mode"
            || field_valid(field, member, matches!(group, Group::Files))
    }) && group.rules(value)
}

fn repair_entry(
    entry: &JsValue,
    group: Group,
    say: impl Fn(&str) -> PluginIssue,
) -> (Option<JsValue>, Option<PluginIssue>) {
    let identity = group.identity();
    let Some(members) = entry
        .as_object()
        .filter(|_| group.identity_valid(entry.get(identity)))
    else {
        return (
            None,
            Some(say(&format!(
                "The entry has no valid \"{identity}\"; the entry is skipped"
            ))),
        );
    };
    if !mode_in(entry, &MODES) {
        return (
            None,
            Some(say(
                "The entry has an unknown \"mode\"; the entry is skipped",
            )),
        );
    }
    if is_deny_entry(entry) || is_whole(members, entry, group) {
        return (Some(entry.clone()), None);
    }
    let degraded = JsObject::from([
        (
            identity.to_owned(),
            entry.get(identity).cloned().unwrap_or_default(),
        ),
        ("mode".to_owned(), JsValue::String("deny".to_owned())),
    ]);
    (
        Some(JsValue::Object(degraded)),
        Some(say(&format!(
            "The entry is invalid; it is degraded to mode \"deny\" so the credential stays blocked until you fix it{}",
            group.degraded_note()
        ))),
    )
}

fn repair_group(
    at: &RepairAt,
    value: &JsValue,
    group: Group,
) -> (Option<Vec<JsValue>>, Vec<PluginIssue>) {
    let field = format!(".credentials.{}", group.list());
    if value.is_undefined() {
        return (None, Vec::new());
    }
    let Some(items) = value.as_array() else {
        return (
            None,
            vec![note(
                at,
                &field,
                &format!("\"{}\" must be a list; the list is dropped", group.list()),
            )],
        );
    };
    let mut kept = Vec::new();
    let mut found = Vec::new();
    for (index, entry) in items.iter().enumerate() {
        let (one, said) = repair_entry(entry, group, |message| {
            note(at, &format!("{field}[{index}]"), message)
        });
        kept.extend(one);
        found.extend(said);
    }
    (Some(kept), found)
}

/// The `credentials` block after the per-entry repair, with what was said about it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RepairedCredentials {
    /// The block.
    pub value: JsValue,
    /// What was said about it.
    pub found: Vec<PluginIssue>,
}

/// The `credentials` block after the per-entry repair, with what was said about it.
#[must_use]
pub fn repair_credentials(at: &RepairAt, block: &JsValue) -> RepairedCredentials {
    let Some(members) = block.as_object() else {
        return RepairedCredentials {
            value: JsValue::Undefined,
            found: vec![note(
                at,
                ".credentials",
                "\"credentials\" must be an object; the block is dropped",
            )],
        };
    };
    let mut value: JsObject = property_order(members)
        .into_iter()
        .filter(|(name, _)| !GROUPS.iter().any(|one| one.list() == name.as_str()))
        .map(|(name, member)| (name.clone(), member.clone()))
        .collect();
    let mut found = Vec::new();
    for group in GROUPS {
        let list = block.get(group.list()).cloned().unwrap_or_default();
        let (kept, said) = repair_group(at, &list, group);
        if let Some(entries) = kept {
            value.insert(group.list().to_owned(), JsValue::Array(entries));
        }
        found.extend(said);
    }
    RepairedCredentials {
        value: JsValue::Object(in_property_order(value)),
        found,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_env_name, is_extract};
    use crate::js::JsValue;

    #[test]
    fn an_extract_needs_a_capturing_group() {
        assert!(is_extract(&JsValue::String("a(b)".to_owned())));
        assert!(!is_extract(&JsValue::String("ab".to_owned())));
        assert!(!is_extract(&JsValue::String("(".to_owned())));
        assert!(is_env_name("_A1"));
        assert!(!is_env_name("1A"));
    }
}
