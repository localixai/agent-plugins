//! Ported from `src/settings/repair/hooks.ts`; only the core uses it now, so it has no wrapper.
//!
//! `hooks` of a settings file, read with the hooks reader so a settings file and a plugin's
//! `hooks.json` agree on what loads.

use super::support::{RepairAt, Repaired, kept};
use crate::format::hooks_settings::{HooksReadOptions, HooksSettings, read_hooks_settings};
use crate::format::issues::{PluginIssue, Severity, blocks_loading};
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;

/// The hooks a reader kept, as the value of the key.
fn value_of(hooks: HooksSettings) -> JsValue {
    JsValue::Object(
        hooks
            .into_iter()
            .map(|(event, groups)| (event, JsValue::Array(groups)))
            .collect(),
    )
}

/// The members of a group with `hooks` set to `handlers` (`{ ...group, hooks }`).
fn with_hooks(group: &JsObject, handlers: Vec<JsValue>) -> JsValue {
    let mut members = group.clone();
    members.insert("hooks".to_owned(), JsValue::Array(handlers));
    JsValue::Object(members)
}

/// Whether the hooks reader accepts `groups` as the matcher groups of `event` (no error).
fn loads(event: &str, groups: Vec<JsValue>) -> bool {
    let raw = JsValue::Object(JsObject::from([(event.to_owned(), JsValue::Array(groups))]));
    !blocks_loading(&read_hooks_settings(&raw, HooksReadOptions::default()).issues)
}

/// The matcher group with only the handlers that load: none when the group itself (its shape, its
/// `matcher`, its `hooks` array) cannot load, one otherwise.
fn surviving_group(event: &str, group: &JsValue) -> Vec<JsValue> {
    let (Some(members), Some(JsValue::Array(hooks))) = (
        group.as_object().filter(|_| is_record(group)),
        group.get("hooks"),
    ) else {
        return Vec::new();
    };
    if !loads(event, vec![with_hooks(members, Vec::new())]) {
        return Vec::new();
    }
    let handlers: Vec<JsValue> = hooks
        .iter()
        .filter(|handler| loads(event, vec![with_hooks(members, vec![(*handler).clone()])]))
        .cloned()
        .collect();
    vec![with_hooks(members, handlers)]
}

/// Every event whose value is a list, with the groups and handlers that load.
fn surviving_events(raw: &JsObject) -> JsValue {
    JsValue::Object(
        property_order(raw)
            .into_iter()
            .filter_map(|(event, groups)| {
                let JsValue::Array(groups) = groups else {
                    return None;
                };
                let kept: Vec<JsValue> = groups
                    .iter()
                    .flat_map(|group| surviving_group(event, group))
                    .collect();
                Some((event.clone(), JsValue::Array(kept)))
            })
            .collect(),
    )
}

/// The reader's issues, each tied to the source; an error becomes the warning that its entry is dropped.
fn source_issues(issues: &[PluginIssue], at: &RepairAt, drops_entries: bool) -> Vec<PluginIssue> {
    issues
        .iter()
        .map(|issue| {
            let path = format!("{}:{}", at.scope.name(), issue.path);
            if !drops_entries || issue.severity != Severity::Error {
                return PluginIssue {
                    path,
                    ..issue.clone()
                };
            }
            PluginIssue {
                severity: Severity::Warning,
                path,
                message: format!("{}; the entry is skipped", issue.message),
                guidance: None,
            }
        })
        .collect()
}

/// `hooks`, read with the hooks reader so a settings file and a plugin's `hooks.json` agree on what
/// loads. The policy tier (managed, parent) ignores the whole field on any error: errors.md says
/// "when a line lists the source's `hooks` field as ignored, none of that source's hooks run". Any
/// other source skips only the entries that fail, handler by handler, and keeps the rest of the file
/// in effect (settings.md, "Fix a broken settings file": "only individual entries fail, such as ...
/// an unknown hook event name. Claude Code skips those values and keeps the rest of the file in
/// effect"). A `hooks` value that is not an object leaves nothing to keep. Each issue names the scope.
#[must_use]
pub fn hooks(value: &JsValue, at: &RepairAt) -> Repaired {
    let read = read_hooks_settings(value, HooksReadOptions::default());
    if !blocks_loading(&read.issues) {
        return kept(value_of(read.hooks), source_issues(&read.issues, at, false));
    }
    if at.closed {
        return kept(JsValue::Undefined, source_issues(&read.issues, at, false));
    }
    let issues = source_issues(&read.issues, at, true);
    let Some(members) = value.as_object().filter(|_| is_record(value)) else {
        return kept(JsValue::Undefined, issues);
    };
    let surviving = surviving_events(members);
    kept(
        value_of(read_hooks_settings(&surviving, HooksReadOptions::default()).hooks),
        issues,
    )
}
