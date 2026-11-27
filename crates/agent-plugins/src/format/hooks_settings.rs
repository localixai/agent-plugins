//! Port of `src/format/hooks-settings.ts`, switched.
//!
//! Reading hook configuration (hooks.md, "Configuration"): three levels of nesting, an event, its
//! matcher groups, and each group's handlers. A hooks file (`hooks/hooks.json`) wraps the event map
//! in a top-level `"hooks"` key; an inline object (the manifest's `hooks`, a skill's frontmatter,
//! the `hooks` of `settings.json`) is the event map itself. A file that fails the hooks schema does
//! not load (`Failed to load hooks from <path>: <reason>`), so any error leaves `hooks` empty; the
//! caller, which knows the path, adds it to the reason.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::hooks_handlers::read_handler;
use super::issues::{PluginIssue, Severity, blocks_loading, error, warning};
use crate::js::regexp::RegExp;
use crate::js::{JsObject, JsValue};
use crate::protocol::hook_events::HOOK_EVENTS;
use crate::protocol::is_record::is_record;

/// The official `HooksSettings`: matcher groups by event.
pub type HooksSettings = IndexMap<String, Vec<JsValue>>;

/// What reading hooks gave.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HooksRead {
    /// Official shape. Empty when an error keeps the file from loading.
    pub hooks: HooksSettings,
    /// What was found.
    pub issues: Vec<PluginIssue>,
    /// Whether the plugin holding these hooks does not load at all (oracle 2.1.319, read `lenient`
    /// only): a guard hook that cannot load "may be what guards the permissions declared beside it,
    /// so nothing it sits in is applied".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejects_plugin: Option<bool>,
}

fn is_event(name: &str) -> bool {
    HOOK_EVENTS.contains(&name)
}

/// Events the docs list as "no matcher support" (hooks.md, "Matcher patterns"): a `matcher` there
/// "is silently ignored", so a pattern that does not compile is not worth a warning.
const NO_MATCHER_EVENTS: &[&str] = &[
    "CwdChanged",
    "UserPromptSubmit",
    "PostToolBatch",
    "Stop",
    "TeammateIdle",
    "TaskCreated",
    "TaskCompleted",
    "WorktreeCreate",
    "WorktreeRemove",
    "MessageDisplay",
];

/// An unanchored pattern that does not compile matches nothing (judgment): say so once, at load.
fn matcher_issues(matcher: Option<&JsValue>, path: &str, ignored: bool) -> Vec<PluginIssue> {
    let Some(matcher) = matcher.filter(|matcher| !matcher.is_undefined()) else {
        return Vec::new();
    };
    let Some(matcher) = matcher.as_str() else {
        return vec![error(path, "Invalid \"matcher\": expected a string")];
    };
    if ignored {
        return Vec::new();
    }
    // "*" is match-all, not a pattern; `rules` holds the grammar, which layer 2 cannot import.
    if matcher == "*" {
        return Vec::new();
    }
    match RegExp::new(matcher, "") {
        Ok(_) => Vec::new(),
        Err(_) => vec![warning(
            path,
            &format!(
                "The matcher \"{matcher}\" is not a valid regular expression and matches nothing"
            ),
        )],
    }
}

/// How a hooks object treats an entry it cannot read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HooksReadOptions {
    /// Drop the entry with a warning instead of failing the whole object, except under the guarded
    /// events: a guard that silently went missing could leave what it protected open. Plugin hooks
    /// load this way (oracle 2.1.319, docs/SPEC.md "Sources of truth"); `settings.json` hooks do not
    /// (settings.md: a managed event that is not a list ignores the whole field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lenient: Option<bool>,
}

impl HooksReadOptions {
    fn lenient(self) -> bool {
        self.lenient == Some(true)
    }
}

const GUARDED_EVENTS: &[&str] = &["PreToolUse", "PermissionRequest"];

fn is_guarded(event: &str) -> bool {
    GUARDED_EVENTS.contains(&event)
}

/// What an entry's problems amount to under its event: the entry is dropped, with a warning unless it guards.
fn entry_issues(
    event: &str,
    issues: Vec<PluginIssue>,
    options: HooksReadOptions,
) -> Vec<PluginIssue> {
    if !options.lenient() || is_guarded(event) {
        return issues;
    }
    issues
        .into_iter()
        .map(|found| match found.severity {
            Severity::Error => warning(
                &found.path,
                &format!("{}; the entry is ignored", found.message),
            ),
            Severity::Warning => found,
        })
        .collect()
}

fn guard_failure(event: &str, options: HooksReadOptions) -> Option<bool> {
    (options.lenient() && is_guarded(event)).then_some(true)
}

/// A group read, and what was found.
struct GroupRead {
    group: Option<JsValue>,
    issues: Vec<PluginIssue>,
}

fn read_group(raw: &JsValue, path: &str, ignores_matcher: bool) -> GroupRead {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return GroupRead {
            group: None,
            issues: vec![error(path, "A matcher group must be an object")],
        };
    };
    let matcher = members.get("matcher");
    let mut issues = matcher_issues(matcher, &format!("{path}.matcher"), ignores_matcher);
    // A group whose matcher cannot be read is dropped: loaded without it, it would match everything.
    if blocks_loading(&issues) {
        return GroupRead {
            group: None,
            issues,
        };
    }
    // Another key of a group is dropped without a warning (Judgment J7.10, oracle 2.1.325).
    let Some(handlers_raw) = members.get("hooks").and_then(JsValue::as_array) else {
        issues.push(error(
            &format!("{path}.hooks"),
            "Invalid \"hooks\": expected an array of handlers",
        ));
        return GroupRead {
            group: None,
            issues,
        };
    };
    let read: Vec<_> = handlers_raw
        .iter()
        .enumerate()
        .map(|(index, handler)| read_handler(handler, &format!("{path}.hooks[{index}]")))
        .collect();
    let mut handlers = Vec::new();
    for one in read {
        issues.extend(one.issues);
        handlers.extend(one.handler);
    }
    let mut group = JsObject::new();
    if let Some(JsValue::String(matcher)) = matcher {
        group.insert("matcher".to_owned(), matcher.as_str().into());
    }
    group.insert("hooks".to_owned(), JsValue::Array(handlers));
    GroupRead {
        group: Some(JsValue::Object(group)),
        issues,
    }
}

/// An event read.
struct EventRead {
    groups: Option<Vec<JsValue>>,
    issues: Vec<PluginIssue>,
    /// A guard event that could not be read: under `lenient`, the plugin does not load.
    rejects_plugin: Option<bool>,
}

fn read_event(event: &str, raw: &JsValue, options: HooksReadOptions) -> EventRead {
    let path = format!("hooks.{event}");
    let known = is_event(event);
    // Judgment (spec/judgments.md): an unknown event name is warned about, not an error; a plugin's
    // hooks do not load it (oracle 2.1.319), `settings.json` keeps it as written.
    let mut issues = if known {
        Vec::new()
    } else {
        vec![warning(&path, &format!("\"{event}\" is not a hook event"))]
    };
    let nothing = |issues| EventRead {
        groups: None,
        issues,
        rejects_plugin: None,
    };
    if options.lenient() && !known {
        return nothing(issues);
    }
    if options.lenient() && matches!(raw, JsValue::Null) {
        return nothing(issues);
    }
    let Some(groups_raw) = raw.as_array() else {
        let invalid = error(
            &path,
            &format!("Invalid \"{event}\": expected an array of matcher groups"),
        );
        issues.extend(entry_issues(event, vec![invalid], options));
        return EventRead {
            groups: None,
            issues,
            rejects_plugin: guard_failure(event, options),
        };
    };
    let ignores_matcher = NO_MATCHER_EVENTS.contains(&event);
    let mut found = Vec::new();
    let mut groups = Vec::new();
    for (index, group) in groups_raw.iter().enumerate() {
        let one = read_group(group, &format!("{path}[{index}]"), ignores_matcher);
        found.extend(one.issues);
        groups.extend(one.group);
    }
    let rejects_plugin = if blocks_loading(&found) {
        guard_failure(event, options)
    } else {
        None
    };
    issues.extend(entry_issues(event, found, options));
    EventRead {
        groups: Some(groups),
        issues,
        rejects_plugin,
    }
}

/// Reads an event map: the `hooks` object of `settings.json`, or an inline `hooks` object.
#[must_use]
pub fn read_hooks_settings(raw: &JsValue, options: HooksReadOptions) -> HooksRead {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return HooksRead {
            hooks: HooksSettings::new(),
            issues: vec![error("hooks", "The hooks configuration must be an object")],
            rejects_plugin: None,
        };
    };
    let mut entries = HooksSettings::new();
    let mut issues = Vec::new();
    let mut rejects_plugin = false;
    for (event, value) in members {
        let read = read_event(event, value, options);
        issues.extend(read.issues);
        rejects_plugin = rejects_plugin || read.rejects_plugin == Some(true);
        if let Some(groups) = read.groups {
            entries.insert(event.clone(), groups);
        }
    }
    // `Object.fromEntries` defines own properties, so a JSON-parsed `__proto__` event stays a key.
    let hooks = if blocks_loading(&issues) {
        HooksSettings::new()
    } else {
        entries
    };
    HooksRead {
        hooks,
        issues,
        rejects_plugin: rejects_plugin.then_some(true),
    }
}

/// Top-level keys of a hooks file that are not event maps.
const FILE_KEYS: &[&str] = &["hooks", "description", "modules"];

/// A matcher group with handlers: an object whose `hooks` is a non-empty list.
fn holds_handlers(group: &JsValue) -> bool {
    is_record(group)
        && group
            .get("hooks")
            .and_then(JsValue::as_array)
            .is_some_and(|hooks| !hooks.is_empty())
}

/// Whether a hooks file holds, outside its `hooks` object, what may be a guard (oracle 2.1.319): a
/// `PreToolUse` or `PermissionRequest` key that is anything but null or an empty list, or a key that
/// is no hook event holding a matcher group with handlers (`_comment: ["text"]` is harmless). Another known event there is ignored.
fn stray_guard(raw: &JsObject) -> bool {
    raw.iter().any(|(key, value)| {
        if FILE_KEYS.contains(&key.as_str()) {
            return false;
        }
        if is_guarded(key) {
            return !matches!(value, JsValue::Null) && !value.as_array().is_some_and(Vec::is_empty);
        }
        !is_event(key)
            && value
                .as_array()
                .is_some_and(|groups| groups.iter().any(holds_handlers))
    })
}

/// Reads `hooks/hooks.json`: `{ "hooks": { <event>: [ { matcher?, hooks: [ <handler> ] } ] } }`.
/// Other top-level keys are ignored: `modules` belongs to the mods reader (components.md, "Hooks"),
/// and a file with `modules` alone holds no hooks.
#[must_use]
pub fn read_hooks_file(raw: &JsValue, options: HooksReadOptions) -> HooksRead {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return HooksRead {
            hooks: HooksSettings::new(),
            issues: vec![error("hooks", "The hooks file must be a JSON object")],
            rejects_plugin: None,
        };
    };
    let rejects = options.lenient() && stray_guard(members);
    if let Some(wrapped) = members.get("hooks") {
        return read_wrapped(wrapped, rejects, options);
    }
    // A mod-only file (`modules` with no `hooks`) is valid (mods/reference.md, "Files"): no hooks.
    if members.contains_key("modules") && !rejects {
        return HooksRead::default();
    }
    HooksRead {
        hooks: HooksSettings::new(),
        rejects_plugin: rejects.then_some(true),
        issues: vec![error(
            "hooks",
            "The hooks file has no top-level \"hooks\" key: a file with only the event map, without that wrapper, fails to load",
        )],
    }
}

/// The `hooks` object of a hooks file; `rejects`: the file also holds a stray list outside it.
fn read_wrapped(wrapped: &JsValue, rejects: bool, options: HooksReadOptions) -> HooksRead {
    let lenient = options.lenient();
    let empty = JsValue::Object(JsObject::new());
    let given = if matches!(wrapped, JsValue::Null) && lenient {
        &empty
    } else {
        wrapped
    };
    let read = read_hooks_settings(given, options);
    let wrong_shape = lenient && !matches!(wrapped, JsValue::Null) && !is_record(wrapped);
    if !(rejects || wrong_shape || read.rejects_plugin == Some(true)) {
        return read;
    }
    let mut issues = Vec::new();
    if rejects {
        issues.push(error(
            "hooks",
            "An event list is declared at the top level, outside the \"hooks\" object",
        ));
    }
    issues.extend(read.issues);
    HooksRead {
        hooks: read.hooks,
        issues,
        rejects_plugin: Some(true),
    }
}

crate::registry::exports! {
    "format/hooks-settings" {
        "readHooksSettings" => ok |raw: JsValue, options: Option<HooksReadOptions>|
            read_hooks_settings(&raw, options.unwrap_or_default());
        "readHooksFile" => ok |raw: JsValue, options: Option<HooksReadOptions>|
            read_hooks_file(&raw, options.unwrap_or_default());
    }
}
