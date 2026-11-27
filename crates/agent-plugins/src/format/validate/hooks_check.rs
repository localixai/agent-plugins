//! Port of `src/format/validate/hooks-check.ts`, switched.
//!
//! The hook configuration checks of `claude plugin validate` (oracle 2.1.319), for the `hooks`
//! object of `hooks/hooks.json` and the inline one of `plugin.json`. An entry that cannot load is
//! dropped at runtime and reported as a warning, except under `PreToolUse` and `PermissionRequest`:
//! a guard that silently went missing could leave the permissions beside it unprotected, so there
//! it is an error.

use serde::Deserialize;

use super::finding::{Finding, error_at, warning_at};
use super::hook_handlers::{handler_advice, handler_problem};
use super::received::{kind_of, with_article};
use crate::js::{JsObject, JsValue};
use crate::protocol::hook_events::HOOK_EVENTS;
use crate::protocol::is_record::is_record;

const GUARDED: &[&str] = &["PreToolUse", "PermissionRequest"];

const IGNORED: &str = "; entry ignored at runtime";
/// Why a guard that cannot load is an error.
pub const GUARDED_TAIL: &str = " — a PreToolUse/PermissionRequest hook that cannot be loaded may be what guards the permissions declared beside it, so nothing it sits in is applied until the entry is fixed or removed";

/// One problem with an entry, worded for its event.
fn entry_finding(event: &str, place: &str, problem: &str) -> Finding {
    if GUARDED.contains(&event) {
        error_at("hooks", &format!("{place}: {problem}{GUARDED_TAIL}"))
    } else {
        warning_at("hooks", &format!("{place}: {problem}{IGNORED}"))
    }
}

fn handler_findings(event: &str, place: &str, handlers: &[JsValue], advice: bool) -> Vec<Finding> {
    let mut found = Vec::new();
    for (index, handler) in handlers.iter().enumerate() {
        let at = format!("{place}.hooks.{index}");
        if let Some(problem) = handler_problem(handler) {
            found.push(entry_finding(event, &at, &problem));
            continue;
        }
        let shell = if advice {
            handler_advice(handler)
        } else {
            None
        };
        if let Some(shell) = shell {
            found.push(warning_at(&format!("hooks.{event}"), &shell));
        }
    }
    found
}

fn group_findings(event: &str, place: &str, group: &JsValue, advice: bool) -> Vec<Finding> {
    if !is_record(group) {
        let message = format!(
            "Hook matcher must be an object; received {}",
            with_article(group)
        );
        return vec![entry_finding(event, place, &message)];
    }
    let matcher = group.get("matcher").unwrap_or(&JsValue::Undefined);
    let hooks = group.get("hooks").unwrap_or(&JsValue::Undefined);
    let Some(handlers) = hooks.as_array() else {
        let message = format!(
            "Hook matcher \"hooks\" must be an array of hook entries; received {}",
            with_article(hooks)
        );
        return vec![entry_finding(event, place, &message)];
    };
    if !matcher.is_undefined() && !matcher.is_string() {
        let message = format!(
            "Invalid hook matcher (matcher: Invalid input: expected string, received {})",
            kind_of(matcher)
        );
        return vec![entry_finding(event, place, &message)];
    }
    handler_findings(event, place, handlers, advice)
}

fn event_findings(site: &HooksSite, event: &str, groups: &JsValue) -> Vec<Finding> {
    let place = format!("{}.{event}", site.prefix);
    if !HOOK_EVENTS.contains(&event) {
        return vec![warning_at(
            "hooks",
            &format!("{place}: unknown hook event{IGNORED}"),
        )];
    }
    let Some(groups) = groups.as_array() else {
        let message = format!(
            "must be an array of matchers; received {}",
            with_article(groups)
        );
        return vec![entry_finding(event, &place, &message)];
    };
    groups
        .iter()
        .enumerate()
        .flat_map(|(index, group)| {
            group_findings(event, &format!("{place}.{index}"), group, site.advice)
        })
        .collect()
}

/// Where a hooks object sits and whether its shell commands are advised on.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct HooksSite {
    /// How the oracle names it: `hooks`, or `hooks.0` for one map of a list.
    pub prefix: String,
    /// The unquoted-placeholder warning applies to `hooks/hooks.json` only, not to hooks written inline.
    pub advice: bool,
}

fn file_site() -> HooksSite {
    HooksSite {
        prefix: "hooks".to_owned(),
        advice: true,
    }
}

/// The findings of one hooks object, event by event.
#[must_use]
pub fn hooks_map_findings(map: &JsObject, site: Option<&HooksSite>) -> Vec<Finding> {
    let fallback = file_site();
    let site = site.unwrap_or(&fallback);
    map.iter()
        .flat_map(|(event, groups)| event_findings(site, event, groups))
        .collect()
}

crate::registry::exports! {
    "format/validate/hooks-check" {
        "GUARDED_TAIL" => ok || GUARDED_TAIL;
        "hooksMapFindings" => ok |map: JsObject, site: Option<HooksSite>| hooks_map_findings(&map, site.as_ref());
    }
}
