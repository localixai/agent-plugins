//! Port of `src/settings/permissions/auto-mode.ts`, switched.
//!
//! The auto mode keys: the `autoMode` block that tells the classifier what the organization trusts,
//! and the three Booleans around it (settings-reference.md, `autoMode`,
//! `autoMode.classifyAllShell`, `useAutoModeDuringPlan`, `skipAutoPermissionPrompt`).

use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::js::value::JsObject;
use crate::js::{JsValue, in_property_order};
use crate::settings::permissions::read::flag_value;
use crate::settings::repair::support::{
    RepairAt, Repaired, dropped, kept, strip_invalid_now, where_,
};
use crate::settings::view::PolicyView;

/// The prose rule lists of the block, in the docs' order.
pub const AUTO_MODE_LISTS: [&str; 4] = ["environment", "allow", "soft_deny", "hard_deny"];

/// The lists whose loss withholds the others: "a `soft_deny` or `hard_deny` list that can't be
/// read".
const RESTRICTING: [&str; 2] = ["soft_deny", "hard_deny"];
/// The lists withheld while a restricting list cannot be read.
const GRANTING: [&str; 2] = ["allow", "environment"];

fn field_at(at: &RepairAt, field: &str) -> String {
    where_(at, &format!(".{field}"))
}

/// The one list of the block: its entries and whether anything was lost reading it.
struct ListRead {
    entries: Option<Vec<JsValue>>,
    lost: bool,
    issues: Vec<PluginIssue>,
}

fn read_list(raw: &JsValue, at: &RepairAt, field: &str) -> ListRead {
    let JsValue::Array(list) = raw else {
        return ListRead {
            entries: None,
            lost: true,
            issues: vec![warning(
                &field_at(at, field),
                &format!("\"autoMode.{field}\" must be a list; it is dropped"),
            )],
        };
    };
    let read = strip_invalid_now(
        list,
        at,
        |entry| entry.as_str().map(|_| entry.clone()),
        "Expected a text",
    );
    ListRead {
        entries: Some(read.entries),
        lost: !read.issues.is_empty(),
        issues: read.issues,
    }
}

fn withheld(fields: &mut JsObject, lost_restriction: &[String], at: &RepairAt) -> Vec<PluginIssue> {
    if !at.closed || lost_restriction.is_empty() {
        return Vec::new();
    }
    let names: Vec<String> = lost_restriction
        .iter()
        .map(|name| format!("\"autoMode.{name}\""))
        .collect();
    let names = names.join(" and ");
    GRANTING
        .iter()
        .filter_map(|list| {
            fields.get(*list).filter(|value| !value.is_undefined())?;
            fields.shift_remove(*list);
            Some(warning(
                &field_at(at, list),
                &format!(
                    "\"autoMode.{list}\" is withheld: {names} could not be read or lost an invalid entry"
                ),
            ))
        })
        .collect()
}

/// What reading one field of the block gave.
struct FieldRead {
    value: JsValue,
    issues: Vec<PluginIssue>,
    /// A restricting list that could not be read whole.
    lost_restriction: bool,
}

fn shell_field(raw: &JsValue, at: &RepairAt) -> FieldRead {
    let read = flag_value(raw, at, "autoMode.classifyAllShell", true);
    FieldRead {
        value: read.value,
        issues: read.issues,
        lost_restriction: false,
    }
}

fn list_field(name: &str, raw: &JsValue, at: &RepairAt) -> FieldRead {
    let read = read_list(raw, at, name);
    FieldRead {
        value: read.entries.map_or(JsValue::Undefined, JsValue::Array),
        issues: read.issues,
        lost_restriction: read.lost && RESTRICTING.contains(&name),
    }
}

/// `FIELDS[name]`: `classifyAllShell`, then one reader per list.
fn read_field(name: &str, raw: &JsValue, at: &RepairAt) -> FieldRead {
    if name == "classifyAllShell" {
        return shell_field(raw, at);
    }
    if AUTO_MODE_LISTS.contains(&name) {
        return list_field(name, raw, at);
    }
    FieldRead {
        value: raw.clone(),
        issues: Vec::new(),
        lost_restriction: false,
    }
}

/// `autoMode` of one source, repaired per field.
#[must_use]
pub fn auto_mode_block(value: &JsValue, at: &RepairAt) -> Repaired {
    let JsValue::Object(members) = value else {
        return dropped(at, &format!("\"{}\" must be an object", at.key));
    };
    let reads: Vec<(String, FieldRead)> = in_property_order(members.clone())
        .into_iter()
        .filter(|(_, raw)| !matches!(raw, JsValue::Null))
        .map(|(name, raw)| {
            let read = read_field(&name, &raw, at);
            (name, read)
        })
        .collect();
    let mut fields: JsObject = reads
        .iter()
        .filter(|(_, read)| !read.value.is_undefined())
        .map(|(name, read)| (name.clone(), read.value.clone()))
        .collect();
    let lost: Vec<String> = reads
        .iter()
        .filter(|(_, read)| read.lost_restriction)
        .map(|(name, _)| name.clone())
        .collect();
    let mut issues: Vec<PluginIssue> = reads
        .into_iter()
        .flat_map(|(_, read)| read.issues)
        .collect();
    issues.extend(withheld(&mut fields, &lost, at));
    kept(JsValue::Object(fields), issues)
}

/// A Boolean the user can only opt into, which the policy tier fails closed on: unreadable it
/// reads as `false` (the notice or the dialog still shows; the plan-mode classifier is off).
#[must_use]
pub fn consent_boolean(value: &JsValue, at: &RepairAt) -> Repaired {
    let read = flag_value(value, at, &at.key, false);
    kept(read.value, read.issues)
}

/// The classifier's prose lists, in the docs' order.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AutoModeLists {
    /// `environment`.
    pub environment: Vec<JsValue>,
    /// `allow`.
    pub allow: Vec<JsValue>,
    /// `soft_deny`.
    pub soft_deny: Vec<JsValue>,
    /// `hard_deny`.
    pub hard_deny: Vec<JsValue>,
}

/// The auto mode settings in force.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoModeSettings {
    /// The classifier's prose lists, every file's entries joined, highest source first.
    /// `$defaults` is kept as written.
    pub lists: AutoModeLists,
    /// "Send every shell command through the classifier": a `true` from user, `--settings` or
    /// managed applies.
    pub classify_all_shell: bool,
    /// Plan mode uses the classifier: on unless a `false` from managed, `--settings`, user or local
    /// turns it off.
    pub use_auto_mode_during_plan: bool,
    /// The one-time notice of auto mode is skipped.
    pub skip_auto_permission_prompt: bool,
}

/// The auto mode settings of a view (docs/API.md, "Settings").
pub async fn auto_mode_settings(view: &dyn PolicyView) -> AutoModeSettings {
    let block = view.get("autoMode").await;
    let list = |name: &str| -> Vec<JsValue> {
        match block.as_ref().and_then(|block| block.get(name)) {
            Some(JsValue::Array(entries)) => entries.clone(),
            _ => Vec::new(),
        }
    };
    AutoModeSettings {
        lists: AutoModeLists {
            environment: list("environment"),
            allow: list("allow"),
            soft_deny: list("soft_deny"),
            hard_deny: list("hard_deny"),
        },
        classify_all_shell: view.get("autoMode.classifyAllShell").await
            == Some(JsValue::Bool(true)),
        use_auto_mode_during_plan: view.get("useAutoModeDuringPlan").await
            != Some(JsValue::Bool(false)),
        skip_auto_permission_prompt: view.get("skipAutoPermissionPrompt").await
            == Some(JsValue::Bool(true)),
    }
}

crate::registry::exports! {
    "settings/permissions/auto-mode" {
        "AUTO_MODE_LISTS" => ok || AUTO_MODE_LISTS;
    }
}
