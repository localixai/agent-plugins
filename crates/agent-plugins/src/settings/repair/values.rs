//! Port of `src/settings/repair/values.ts`, switched.
//!
//! The repairs of the plain values: a Boolean, a lock, a text, an object, and the maps of plugins,
//! marketplaces and skills.

use super::support::{
    EntryAt, RepairAt, Repaired, dropped, entry_dropped, failed_closed, is_parent, kept, where_,
};
use crate::format::issues::warning;
use crate::js::{JsObject, JsValue, in_property_order, is_whitespace, property_order};
use crate::protocol::is_record::is_record;
use crate::protocol::settings_keys::{SKILL_OVERRIDE_STATES, WORKFLOW_SIZES};

/// A Boolean: anything else is dropped with a warning, even the text `"true"` (the docs'
/// `disableAllHooks` case).
#[must_use]
pub fn boolean(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_boolean() {
        kept(value.clone(), Vec::new())
    } else {
        dropped(at, &format!("\"{}\" must be true or false", at.key))
    }
}

/// A Boolean lock (`allowManagedHooksOnly`, `disableSkillShellExecution`...). In the policy tier
/// the text `"true"` or `"false"` reads as that Boolean with a notice to drop the quotes, and any
/// other unreadable value reads as `true`, the restrictive one (managed-settings.md, "Keys that
/// fail closed"). Elsewhere an unreadable value is dropped.
#[must_use]
pub fn lock_boolean(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_boolean() {
        return kept(value.clone(), Vec::new());
    }
    if !at.closed {
        return dropped(at, &format!("\"{}\" must be true or false", at.key));
    }
    if let Some(text @ ("true" | "false")) = value.as_str() {
        return kept(
            JsValue::Bool(text == "true"),
            vec![warning(
                &where_(at, ""),
                &format!("\"{}\" is the text \"{text}\": drop the quotes", at.key),
            )],
        );
    }
    failed_closed(at, JsValue::Bool(true), "true")
}

/// `disableAllHooks`. In the SDK's `managedSettings` "any value but `false` and `"false"` binds"
/// as `allowManagedHooksOnly: true`, "because an enterprise configuration often yields a boolean
/// as text", so there an unreadable value reads as `true`, with a warning. Every other source reads
/// a Boolean and drops anything else.
#[must_use]
pub fn hooks_switch(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_boolean() || !is_parent(at) {
        return boolean(value, at);
    }
    if value.as_str() == Some("false") {
        return kept(
            JsValue::Bool(false),
            vec![warning(
                &where_(at, ""),
                &format!("\"{}\" is the text \"false\": drop the quotes", at.key),
            )],
        );
    }
    failed_closed(
        at,
        JsValue::Bool(true),
        "true: it binds as \"allowManagedHooksOnly\": true",
    )
}

/// A text.
#[must_use]
pub fn string(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_string() {
        kept(value.clone(), Vec::new())
    } else {
        dropped(at, &format!("\"{}\" must be a text", at.key))
    }
}

/// An object whose entries the kit reads later, per plugin or per server.
#[must_use]
pub fn record(value: &JsValue, at: &RepairAt) -> Repaired {
    if is_record(value) {
        kept(value.clone(), Vec::new())
    } else {
        dropped(at, &format!("\"{}\" must be an object", at.key))
    }
}

/// `strictPluginOnlyCustomization` is `true` or a list of surfaces. Unreadable in the policy tier
/// it reads as `true`, locking all four; an entry of a list that this version does not know locks
/// nothing, and the customization policy counts it (managed-settings.md), so a list keeps its
/// entries as written.
#[must_use]
pub fn plugin_only(value: &JsValue, at: &RepairAt) -> Repaired {
    if value.is_boolean() || value.is_array() {
        return kept(value.clone(), Vec::new());
    }
    if at.closed {
        failed_closed(
            at,
            JsValue::Bool(true),
            "true: all four kinds of customization are locked",
        )
    } else {
        dropped(
            at,
            &format!("\"{}\" must be true or a list of surfaces", at.key),
        )
    }
}

/// `plugin-name@marketplace-name` (settings-reference.md, `enabledPlugins`):
/// `/^[^@\s]+@[^@\s]+$/`.
fn is_plugin_id(id: &str) -> bool {
    let part = |text: &str| !text.is_empty() && !text.chars().any(is_whitespace);
    id.split_once('@')
        .is_some_and(|(name, origin)| part(name) && part(origin) && !origin.contains('@'))
}

/// The members of a map repair, in `Object.entries` order, split into those `valid` keeps and a
/// warning for each other one.
fn split_entries(
    members: &JsObject,
    at: &RepairAt,
    valid: impl Fn(&str, &JsValue) -> bool,
    reason: impl Fn(&str) -> String,
) -> (JsObject, Vec<crate::format::issues::PluginIssue>, usize) {
    let entries = property_order(members);
    let mut issues = Vec::new();
    let mut kept_entries = JsObject::new();
    for (name, entry) in &entries {
        if valid(name, entry) {
            kept_entries.insert((*name).clone(), (*entry).clone());
        } else {
            issues.push(entry_dropped(
                at,
                &EntryAt::Name((*name).clone()),
                &reason(name),
            ));
        }
    }
    (in_property_order(kept_entries), issues, entries.len())
}

/// `enabledPlugins`: an invalid entry is dropped with a warning and the others stay; a value that
/// is no map of plugin ids, or whose every entry is invalid, is dropped whole
/// (managed-settings.md). The SDK also types a list or an object as the value (version
/// constraints), kept as written.
#[must_use]
pub fn enabled_plugins(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(members) = value.as_object() else {
        return dropped(
            at,
            &format!("\"{}\" must map plugin ids to true or false", at.key),
        );
    };
    let (valid, mut issues, count) = split_entries(
        members,
        at,
        |id, entry| {
            is_plugin_id(id) && (entry.is_boolean() || entry.is_array() || is_record(entry))
        },
        |id| format!("\"{id}\" is not a plugin id mapped to true or false"),
    );
    if count > 0 && valid.is_empty() {
        issues.extend(dropped(at, "Every entry is invalid").issues);
        return kept(JsValue::Undefined, issues);
    }
    kept(JsValue::Object(valid), issues)
}

/// `extraKnownMarketplaces`: a name maps to an object with a `source` object; others are dropped.
#[must_use]
pub fn marketplace_map(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(members) = value.as_object() else {
        return dropped(
            at,
            &format!("\"{}\" must map marketplace names to entries", at.key),
        );
    };
    let (valid, issues, _) = split_entries(
        members,
        at,
        |_, entry| is_record(entry) && entry.get("source").is_some_and(is_record),
        |name| format!("\"{name}\" has no \"source\" object"),
    );
    kept(JsValue::Object(valid), issues)
}

fn is_skill_state(state: &JsValue) -> bool {
    state
        .as_str()
        .is_some_and(|state| SKILL_OVERRIDE_STATES.contains(&state))
}

/// `skillOverrides`: a skill name maps to one of the four states; any other entry is dropped.
#[must_use]
pub fn skill_overrides(value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(members) = value.as_object() else {
        return dropped(
            at,
            &format!("\"{}\" must map skill names to states", at.key),
        );
    };
    let (valid, issues, _) = split_entries(
        members,
        at,
        |_, state| is_skill_state(state),
        |name| {
            format!(
                "\"{name}\" is not one of {}",
                SKILL_OVERRIDE_STATES.join(", ")
            )
        },
    );
    kept(JsValue::Object(valid), issues)
}

/// `workflows.size`: one of the sizes.
#[must_use]
pub fn workflow_size(value: &JsValue, at: &RepairAt) -> Repaired {
    if value
        .as_str()
        .is_some_and(|size| WORKFLOW_SIZES.contains(&size))
    {
        kept(value.clone(), Vec::new())
    } else {
        dropped(
            at,
            &format!(
                "\"{}\" must be one of {}",
                at.key,
                WORKFLOW_SIZES.join(", ")
            ),
        )
    }
}

crate::registry::exports! {
    "settings/repair/values" {
        "boolean" => ok |value: JsValue, at: RepairAt| boolean(&value, &at);
        "lockBoolean" => ok |value: JsValue, at: RepairAt| lock_boolean(&value, &at);
        "hooksSwitch" => ok |value: JsValue, at: RepairAt| hooks_switch(&value, &at);
        "string" => ok |value: JsValue, at: RepairAt| string(&value, &at);
        "record" => ok |value: JsValue, at: RepairAt| record(&value, &at);
        "pluginOnly" => ok |value: JsValue, at: RepairAt| plugin_only(&value, &at);
        "enabledPlugins" => ok |value: JsValue, at: RepairAt| enabled_plugins(&value, &at);
        "marketplaceMap" => ok |value: JsValue, at: RepairAt| marketplace_map(&value, &at);
        "skillOverrides" => ok |value: JsValue, at: RepairAt| skill_overrides(&value, &at);
        "workflowSize" => ok |value: JsValue, at: RepairAt| workflow_size(&value, &at);
    }
}

#[cfg(test)]
mod tests {
    use super::is_plugin_id;

    #[test]
    fn a_plugin_id_has_one_at_and_no_blank() {
        assert!(is_plugin_id("a@b"));
        assert!(!is_plugin_id("a@b@c"));
        assert!(!is_plugin_id("a b@c"));
        assert!(!is_plugin_id("@c"));
        assert!(!is_plugin_id("a@"));
    }
}
