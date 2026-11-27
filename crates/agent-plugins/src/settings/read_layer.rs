//! Port of `src/settings/read-layer.ts`, switched.
//!
//! Turns the JSON of one settings source into a layer every resolver can trust: aliases rewritten
//! to their canonical key, a key the source may not carry dropped with a warning, an unreadable
//! value repaired as the docs say (managed-settings.md, "Find entries Claude Code dropped"). It
//! never throws; every drop and repair is a `PluginIssue`.

use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::js::value::JsObject;
use crate::js::{JsValue, in_property_order};
use crate::protocol::settings_keys::{POLICY_KEYS, marketplace_key_aliases};
use crate::settings::broken_file::{broken_source, scope_label};
use crate::settings::general::checks::general_check;
use crate::settings::general::global_config::GLOBAL_CONFIG_KEYS;
use crate::settings::general::keys::GENERAL_KEYS;
use crate::settings::key_spec::{KEY_SPECS, SPECS, admits, scope_carries};
use crate::settings::layers::{PolicyLayer, PolicyScope};
use crate::settings::paths::{SettingsPath, value_at, with_value_at};
use crate::settings::repair::kinds::repair_of;
use crate::settings::repair::support::{RepairAt, Repaired, note};

/// The policy tier: an unreadable restrictive value there fails closed (managed-settings.md).
const FAILS_CLOSED: [(PolicyScope, bool); 7] = [
    (PolicyScope::Managed, true),
    (PolicyScope::Parent, true),
    (PolicyScope::Flag, false),
    (PolicyScope::Local, false),
    (PolicyScope::Project, false),
    (PolicyScope::AddDir, false),
    (PolicyScope::User, false),
];

fn fails_closed(scope: PolicyScope) -> bool {
    FAILS_CLOSED
        .iter()
        .any(|(one, closed)| *one == scope && *closed)
}

fn is_policy_key(name: &str) -> bool {
    POLICY_KEYS.contains(&name)
}

/// The keys of a source with each alias written as its canonical key; when both spellings are in
/// one file the canonical value is used and the alias ignored (settings-reference.md, "Marketplace
/// key aliases").
fn canonical_entries(raw: &JsObject) -> Vec<(String, JsValue)> {
    let aliases = marketplace_key_aliases();
    crate::js::property_order(raw)
        .into_iter()
        .filter_map(|(name, value)| {
            let Some(canonical) = aliases.get(name.as_str()) else {
                return Some((name.clone(), value.clone()));
            };
            match raw.get(*canonical) {
                None | Some(JsValue::Undefined | JsValue::Null) => {
                    Some(((*canonical).to_owned(), value.clone()))
                }
                Some(_) => None,
            }
        })
        .collect()
}

struct Read {
    entry: Option<(String, JsValue)>,
    issues: Vec<PluginIssue>,
    /// The key was present and unreadable and no value stands for it (`PolicyLayer.invalid`).
    invalid: Option<String>,
}

const fn issues_only(issues: Vec<PluginIssue>) -> Read {
    Read {
        entry: None,
        issues,
        invalid: None,
    }
}

fn is_settings_key(name: &str) -> bool {
    is_policy_key(name) || GENERAL_KEYS.contains(&name)
}

/// The validator of every key: a policy key's names its repair, a general key's is its check.
fn validator_of(key: &str) -> Option<fn(&JsValue, &RepairAt) -> Repaired> {
    if is_policy_key(key) {
        KEY_SPECS.get(key).map(|spec| repair_of(spec.repair))
    } else {
        general_check(key)
    }
}

/// The rows of a field inside a general key's block. A field of a policy key's block
/// (`permissions.defaultMode`) is read by that key's own repair and resolver, which say what a
/// source's value there does.
fn nested_rows() -> impl Iterator<Item = (&'static str, &'static [&'static str])> {
    POLICY_KEYS
        .iter()
        .chain(GENERAL_KEYS.iter())
        .filter_map(|key| {
            let path = SPECS.get(key)?.path?;
            (!is_policy_key(path.first().copied().unwrap_or(""))).then_some((*key, path))
        })
}

fn not_read_from(at: &RepairAt, field: &str, name: &str) -> PluginIssue {
    note(
        at,
        field,
        &format!(
            "\"{name}\" is not read from {}; the key is ignored",
            scope_label(at.scope)
        ),
    )
}

/// The value without the fields whose own row does not admit this source (a managed-only lock
/// inside `sandbox`, `worktree.location` outside user settings), each with a warning: the row's
/// scope line decides, the block's check never repeats it.
fn without_nested_out_of_scope(at: &RepairAt, read: Repaired) -> Repaired {
    nested_rows()
        .filter(|(_, path)| path.first() == Some(&at.key.as_str()))
        .fold(read, |current, (key, path)| {
            let doc = JsValue::Object(JsObject::from([(at.key.clone(), current.value.clone())]));
            let names = SettingsPath::Names(path.iter().map(|name| (*name).to_owned()).collect());
            let found = value_at(&doc, &names);
            if found.is_undefined() || admits(key, at.scope, &found) {
                return current;
            }
            let value = with_value_at(&doc, &names, &JsValue::Undefined)
                .get(&at.key)
                .cloned()
                .unwrap_or_default();
            let mut issues = current.issues;
            let field = path.get(1..).unwrap_or_default().join(".");
            issues.push(not_read_from(at, &format!(".{field}"), key));
            Repaired {
                value,
                issues,
                ..current
            }
        })
}

/// One key of a source: read when its scope line admits it (a general key a source can never
/// carry is left out silently), its value then repaired or checked as its validator says.
fn read_key(scope: PolicyScope, key: &str, value: &JsValue) -> Read {
    let at = RepairAt {
        key: key.to_owned(),
        scope,
        closed: fails_closed(scope),
    };
    if !is_policy_key(key) && !scope_carries(scope, key) {
        return issues_only(Vec::new());
    }
    if !admits(key, scope, value) {
        return issues_only(vec![not_read_from(&at, "", key)]);
    }
    let in_scope = without_nested_out_of_scope(
        &at,
        Repaired {
            value: value.clone(),
            invalid: None,
            issues: Vec::new(),
        },
    );
    let Some(validator) = validator_of(key) else {
        return issues_only(in_scope.issues);
    };
    let repaired = validator(&in_scope.value, &at);
    let mut issues = in_scope.issues;
    issues.extend(repaired.issues);
    Read {
        entry: (!repaired.value.is_undefined()).then(|| (key.to_owned(), repaired.value)),
        issues,
        invalid: (repaired.invalid == Some(true)).then(|| key.to_owned()),
    }
}

fn read_entry(scope: PolicyScope, name: &str, value: &JsValue) -> Read {
    // A `null` removes the key (managed-settings.md, "Keys that fail closed").
    if is_settings_key(name) && matches!(value, JsValue::Null) {
        return issues_only(Vec::new());
    }
    // settings.md, "Change a setting for one session": a global config key is not set from a file.
    if !is_policy_key(name) && GLOBAL_CONFIG_KEYS.contains(&name) {
        return issues_only(vec![warning(
            &format!("{}:{name}", scope.name()),
            &format!(
                "\"{name}\" is a global config key and applies only from ~/.claude.json; the key is ignored"
            ),
        )]);
    }
    if is_settings_key(name) {
        return read_key(scope, name, value);
    }
    Read {
        entry: scope_carries(scope, name).then(|| (name.to_owned(), value.clone())),
        issues: Vec::new(),
        invalid: None,
    }
}

/// A source read to its layer, and what reading it reported.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LayerRead {
    /// The layer.
    pub layer: PolicyLayer,
    /// What was dropped or repaired.
    pub issues: Vec<PluginIssue>,
}

/// An absent source (`undefined`) is an empty layer; a document that is no JSON object is skipped.
#[must_use]
pub fn read_settings_layer(scope: PolicyScope, raw: &JsValue, source: Option<&str>) -> LayerRead {
    let empty = || PolicyLayer {
        scope,
        settings: JsValue::Object(JsObject::new()),
        invalid: None,
    };
    if raw.is_undefined() {
        return LayerRead {
            layer: empty(),
            issues: Vec::new(),
        };
    }
    let JsValue::Object(members) = raw else {
        return LayerRead {
            layer: empty(),
            issues: vec![broken_source(scope, None, source)],
        };
    };
    let reads: Vec<Read> = canonical_entries(members)
        .iter()
        .map(|(name, value)| read_entry(scope, name, value))
        .collect();
    let mut settings = JsObject::new();
    let mut invalid = Vec::new();
    let mut issues = Vec::new();
    for read in reads {
        if let Some((name, value)) = read.entry {
            settings.insert(name, value);
        }
        invalid.extend(read.invalid);
        issues.extend(read.issues);
    }
    LayerRead {
        layer: PolicyLayer {
            scope,
            settings: JsValue::Object(in_property_order(settings)),
            invalid: (!invalid.is_empty()).then_some(invalid),
        },
        issues,
    }
}

crate::registry::exports! {
    "settings/read-layer" {
        "readSettingsLayer" => ok |scope: PolicyScope, raw: JsValue, source: Option<String>| read_settings_layer(scope, &raw, source.as_deref());
    }
}
