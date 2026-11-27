//! Port of `src/settings/repair/support.ts`, switched.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::{JsValue, number_to_string};
use crate::ports::BoxFuture;
use crate::registry::Callback;
use crate::settings::layers::PolicyScope;

/// What a repair knows about the value it reads. `closed` is true for the policy tier (managed and
/// parent settings), where an unreadable value of a restrictive key reads as its restrictive value
/// (managed-settings.md, "Keys that fail closed"); a user, project, local or `--settings` file only
/// loses the value or the entry (settings.md, "Fix a broken settings file").
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RepairAt {
    /// The settings key (`SettingsKey` of `general/keys.ts`).
    pub key: String,
    /// The scope of the file.
    pub scope: PolicyScope,
    /// The policy tier.
    pub closed: bool,
}

/// The value comes from the SDK's `managedSettings`, which the host passes as parent settings.
#[must_use]
pub fn is_parent(at: &RepairAt) -> bool {
    at.scope == PolicyScope::Parent
}

/// The repaired value (`undefined`: the key does not apply) and what was said about it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Repaired {
    /// The repaired value.
    #[serde(skip_serializing_if = "JsValue::is_undefined")]
    pub value: JsValue,
    /// The key was present and unreadable, and no value can stand for what that means: the layer records it (`PolicyLayer.invalid`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<bool>,
    /// What was said about it.
    pub issues: Vec<PluginIssue>,
}

/// `<scope>:<field>`: the source and the field a report names (errors.md: `<source> (<field>)`).
#[must_use]
pub fn where_(at: &RepairAt, field: &str) -> String {
    format!("{}:{}{field}", at.scope.name(), at.key)
}

/// The value as it is, with what was said about it.
#[must_use]
pub const fn kept(value: JsValue, issues: Vec<PluginIssue>) -> Repaired {
    Repaired {
        value,
        issues,
        invalid: None,
    }
}

/// A warning about the value, or about `field` inside it.
#[must_use]
pub fn note(at: &RepairAt, field: &str, message: &str) -> PluginIssue {
    warning(&where_(at, field), message)
}

/// An invalid value that does not apply: dropped with a warning. The one wording of a key that is
/// dropped, the kit's own: errors.md quotes only `crossSessionInbound`'s ("This value was ignored").
#[must_use]
pub fn dropped(at: &RepairAt, reason: &str) -> Repaired {
    Repaired {
        value: JsValue::Undefined,
        issues: vec![note(at, "", &format!("{reason}; the key is ignored"))],
        invalid: None,
    }
}

/// `"<key>" must be <expected>; the key is ignored`.
#[must_use]
pub fn must_be(at: &RepairAt, expected: &str) -> Repaired {
    dropped(at, &format!("\"{}\" must be {expected}", at.key))
}

/// `"<key>" was present but invalid; <effect>`: the report of a value that fails closed (managed-settings.md).
#[must_use]
pub fn present_but_invalid(at: &RepairAt, effect: &str, field: &str) -> PluginIssue {
    note(
        at,
        field,
        &format!("\"{}{field}\" was present but invalid; {effect}", at.key),
    )
}

/// An unreadable value of a restrictive key reads as `effect` until fixed; the report says the key
/// "was present but invalid" and names the value it is treated as (managed-settings.md).
#[must_use]
pub fn failed_closed(at: &RepairAt, value: JsValue, effect: &str) -> Repaired {
    Repaired {
        value,
        issues: vec![present_but_invalid(
            at,
            &format!("it is treated as {effect}"),
            "",
        )],
        invalid: None,
    }
}

/// An unreadable value whose restrictive effect no value expresses: no value is kept, the layer
/// records the key as invalid and the report names what Claude Code does until it is fixed.
#[must_use]
pub fn failed_closed_no_value(at: &RepairAt, effect: &str) -> Repaired {
    Repaired {
        value: JsValue::Undefined,
        issues: vec![present_but_invalid(at, effect, "")],
        invalid: Some(true),
    }
}

/// Where in a list or a map an entry is: its index, or its name.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum EntryAt {
    /// An index of a list.
    Index(f64),
    /// A name in a map.
    Name(String),
}

/// An entry of a list or a map skipped on its own: "Claude Code skips those values and keeps the rest
/// of the file in effect" (settings.md, "Fix a broken settings file").
#[must_use]
pub fn entry_dropped(at: &RepairAt, index: &EntryAt, reason: &str) -> PluginIssue {
    let field = match index {
        EntryAt::Index(number) => format!("[{}]", number_to_string(*number)),
        EntryAt::Name(name) => format!(".{name}"),
    };
    note(at, &field, &format!("{reason}; the entry is skipped"))
}

/// The entries `read` accepted and a warning for each other one.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Stripped {
    /// The entries kept.
    pub entries: Vec<JsValue>,
    /// A warning for each dropped one.
    pub issues: Vec<PluginIssue>,
}

/// What reads one entry of a list: the host's function, so its answer may wait; a Rust host passes a
/// closure, answered at once. `None` is `undefined`: the entry is not valid.
pub trait ReadEntry: Send + Sync {
    /// The entry as it is kept, or `None`.
    fn read<'a>(&'a self, entry: &'a JsValue) -> BoxFuture<'a, Option<JsValue>>;
}

impl<F: Fn(&JsValue) -> Option<JsValue> + Send + Sync> ReadEntry for F {
    fn read<'a>(&'a self, entry: &'a JsValue) -> BoxFuture<'a, Option<JsValue>> {
        Box::pin(std::future::ready(self(entry)))
    }
}

impl ReadEntry for Callback {
    fn read<'a>(&'a self, entry: &'a JsValue) -> BoxFuture<'a, Option<JsValue>> {
        Box::pin(async move {
            let found: JsValue = self.call(vec![entry.clone()]).await;
            (!found.is_undefined()).then_some(found)
        })
    }
}

/// Splits a list into the entries `valid` accepts and a warning for each other one.
pub async fn strip_invalid(
    list: &[JsValue],
    at: &RepairAt,
    read: &dyn ReadEntry,
    reason: &str,
) -> Stripped {
    let mut entries = Vec::new();
    let mut issues = Vec::new();
    for (index, entry) in list.iter().enumerate() {
        match read.read(entry).await {
            None => issues.push(entry_dropped(
                at,
                &EntryAt::Index(f64::from(u32::try_from(index).unwrap_or(u32::MAX))),
                reason,
            )),
            Some(found) => entries.push(found),
        }
    }
    Stripped { entries, issues }
}

/// `strip_invalid` with a reader of the core's own, which answers at once (`ports::now`): what a
/// repair of the core passes.
#[must_use]
pub fn strip_invalid_now(
    list: &[JsValue],
    at: &RepairAt,
    read: impl Fn(&JsValue) -> Option<JsValue> + Send + Sync,
    reason: &str,
) -> Stripped {
    crate::ports::now(strip_invalid(list, at, &read, reason)).unwrap_or_else(|| Stripped {
        entries: Vec::new(),
        issues: Vec::new(),
    })
}

crate::registry::exports! {
    "settings/repair/support" {
        "where" => ok |at: RepairAt, field: Option<String>| where_(&at, field.as_deref().unwrap_or(""));
        "kept" => ok |value: JsValue, issues: Option<Vec<PluginIssue>>| kept(value, issues.unwrap_or_default());
        "note" => ok |at: RepairAt, field: String, message: String| note(&at, &field, &message);
        "dropped" => ok |at: RepairAt, reason: String| dropped(&at, &reason);
        "stripInvalid" => async ok |list: Vec<JsValue>, at: RepairAt, read: Callback, reason: String|
            strip_invalid(&list, &at, &read, &reason).await;
    }
}
