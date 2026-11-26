//! Port of `src/format/core/hooks.ts`, switched.
//!
//! A core package's `hooks.json` (core §7.3.1, §7.3.2): the outer object disables the portable hooks
//! component when it fails (item 2); a member whose event the core does not define, or whose value is
//! not a nonempty array, is skipped (item 3); a handler that is not a closed `{ command, args?,
//! extensions? }` with a valid command token is skipped (item 4). Each skip is reported; the rest
//! loads. The schema is `schemas/1.1.0/hooks.schema.json` (`hooks`, `#/$defs/handlers`, `handler`).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::command::command_problem;
use super::document::{DocumentMembers, document_members};
use super::extensions::{CoreExtensions, extensions_problem, implemented_extensions};
use super::versions::{CoreDocument, CoreVersion};
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsObject, JsValue, property_order};

/// The five events of the core (core §7.3.4–§7.3.8, §11.1 item 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoreHookEvent {
    /// `SessionStart`
    SessionStart,
    /// `UserPromptSubmit`
    UserPromptSubmit,
    /// `PreToolUse`
    PreToolUse,
    /// `PostToolUse`
    PostToolUse,
    /// `Stop`
    Stop,
}

impl CoreHookEvent {
    /// The event as its name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::Stop => "Stop",
        }
    }
}

/// The five events of the core (core §7.3.4–§7.3.8, §11.1 item 6).
pub const CORE_HOOK_EVENTS: [CoreHookEvent; 5] = [
    CoreHookEvent::SessionStart,
    CoreHookEvent::UserPromptSubmit,
    CoreHookEvent::PreToolUse,
    CoreHookEvent::PostToolUse,
    CoreHookEvent::Stop,
];

/// One handler: a command, its arguments and the extensions the host implements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreHookHandler {
    /// The command token.
    pub command: String,
    /// Its arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Only the namespaces the host implements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<CoreExtensions>,
}

/// Each event's handlers, keyed by the event's name, in the file's order.
pub type CoreHooks = IndexMap<String, Vec<CoreHookHandler>>;

/// Where each kept handler sits in its event's array of the file: `positions.<Event>[i]` is the
/// index of `hooks.<Event>[i]`.
pub type CoreHookPositions = IndexMap<String, Vec<usize>>;

/// What reading a `hooks.json` gave.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreHooksResult {
    /// The handlers kept.
    pub hooks: CoreHooks,
    /// The file index of each kept handler, so a later issue names the handler it means.
    pub positions: CoreHookPositions,
    /// The portable hooks component is disabled: the outer object failed.
    pub disabled: bool,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

const FILE: &str = "hooks.json";

fn is_event(name: &str) -> Option<CoreHookEvent> {
    CORE_HOOK_EVENTS
        .iter()
        .copied()
        .find(|event| event.as_str() == name)
}

fn args_problem(value: &JsValue) -> Option<String> {
    if value
        .as_array()
        .is_some_and(|items| items.iter().all(JsValue::is_string))
    {
        None
    } else {
        Some("must be an array of strings".to_owned())
    }
}

fn command_check(value: &JsValue) -> Option<String> {
    command_problem(value).map(str::to_owned)
}

/// The check of one field of a handler.
type FieldCheck = fn(&JsValue) -> Option<String>;

/// The check of each field of a handler, by its name.
const HANDLER_FIELDS: [(&str, FieldCheck); 3] = [
    ("command", command_check),
    ("args", args_problem),
    ("extensions", extensions_problem),
];

fn handler_problem(raw: &JsValue) -> Option<String> {
    let JsValue::Object(members) = raw else {
        return Some("a handler must be an object".to_owned());
    };
    if members.get("command").is_none_or(JsValue::is_undefined) {
        return Some("\"command\" is required".to_owned());
    }
    for (field, value) in property_order(members) {
        let Some((_, check)) = HANDLER_FIELDS.iter().find(|(name, _)| name == field) else {
            return Some(format!("unknown field \"{field}\""));
        };
        if let Some(problem) = check(value) {
            return Some(format!("\"{field}\" {problem}"));
        }
    }
    None
}

fn handler_of(raw: &JsObject, implemented: &[String]) -> CoreHookHandler {
    let extensions = implemented_extensions(
        raw.get("extensions").unwrap_or(&JsValue::Undefined),
        implemented,
    );
    // `handler_problem` passed: `command` is a string and `args`, when present, strings.
    CoreHookHandler {
        command: raw
            .get("command")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
            .to_owned(),
        args: raw.get("args").and_then(JsValue::as_array).map(|args| {
            args.iter()
                .map(|arg| arg.as_str().unwrap_or_default().to_owned())
                .collect()
        }),
        extensions,
    }
}

/// What one event's entries gave.
struct EventHandlers {
    handlers: Vec<CoreHookHandler>,
    positions: Vec<usize>,
    issues: Vec<PluginIssue>,
}

/// The handlers of one event member, each invalid one reported and skipped.
fn event_handlers(
    event: CoreHookEvent,
    entries: &[JsValue],
    implemented: &[String],
) -> EventHandlers {
    let mut read = EventHandlers {
        handlers: Vec::new(),
        positions: Vec::new(),
        issues: Vec::new(),
    };
    for (index, entry) in entries.iter().enumerate() {
        let problem = handler_problem(entry);
        match entry {
            JsValue::Object(members) if problem.is_none() => {
                read.handlers.push(handler_of(members, implemented));
                read.positions.push(index);
            }
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::String(_)
            | JsValue::Array(_)
            | JsValue::Object(_) => read.issues.push(error(
                &format!("hooks.{}[{index}]", event.as_str()),
                &format!("Handler is skipped: {}", problem.unwrap_or_default()),
            )),
        }
    }
    read
}

fn member_issue(name: &str, value: &JsValue) -> Option<PluginIssue> {
    if is_event(name).is_none() {
        return Some(error(
            &format!("hooks.{name}"),
            &format!("Event \"{name}\" is skipped: the core defines no such event"),
        ));
    }
    if value.as_array().is_some_and(|items| !items.is_empty()) {
        None
    } else {
        Some(error(
            &format!("hooks.{name}"),
            &format!("Event \"{name}\" is skipped: its value must be a nonempty array of handlers"),
        ))
    }
}

/// Reads a parsed `hooks.json` against the version `plugin.json` targets.
#[must_use]
pub fn read_core_hooks(
    raw: &JsValue,
    manifest_version: CoreVersion,
    implemented: &[String],
) -> CoreHooksResult {
    let members = match document_members(raw, CoreDocument::Hooks, "hooks", manifest_version) {
        DocumentMembers::Problem(problem) => {
            return CoreHooksResult {
                hooks: IndexMap::new(),
                positions: IndexMap::new(),
                disabled: true,
                issues: vec![error(FILE, &format!("Hooks are disabled: {problem}"))],
            };
        }
        DocumentMembers::Members(members) => members,
    };
    let mut hooks = CoreHooks::new();
    let mut positions = CoreHookPositions::new();
    let mut issues = Vec::new();
    for (name, value) in property_order(&members) {
        if let Some(skipped) = member_issue(name, value) {
            issues.push(skipped);
            continue;
        }
        let (Some(event), Some(entries)) = (is_event(name), value.as_array()) else {
            continue;
        };
        let read = event_handlers(event, entries, implemented);
        issues.extend(read.issues);
        if !read.handlers.is_empty() {
            hooks.insert(name.clone(), read.handlers);
            positions.insert(name.clone(), read.positions);
        }
    }
    CoreHooksResult {
        hooks,
        positions,
        disabled: false,
        issues,
    }
}

crate::registry::exports! {
    "format/core/hooks" {
        "CORE_HOOK_EVENTS" => ok || CORE_HOOK_EVENTS;
        "readCoreHooks" => ok |raw: JsValue, manifest_version: CoreVersion, implemented: Option<Vec<String>>| read_core_hooks(&raw, manifest_version, &implemented.unwrap_or_default());
    }
}
