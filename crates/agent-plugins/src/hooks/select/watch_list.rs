//! Port of `src/hooks/select/watch-list.ts`, switched.
//!
//! hooks.md, "FileChanged": the matcher "serves two roles". Its value builds the watch list: "split
//! on `|` and each segment is registered as a literal filename in the working directory", so
//! `".envrc|.env"` watches exactly those two files and a regular expression such as `^\.env` would
//! watch a file literally named that. An omitted matcher "adds nothing to the watch list", while
//! `"*"` is registered like any value, as a file named `*`.
//!
//! Judgments (spec/judgments.md): an empty segment (`".env|"`, or the empty string) names no file;
//! a handler that never runs (gated off by `disableAllHooks`, trust, a disabled plugin or a managed
//! lock, or of a type `FileChanged` does not support) adds nothing, since no run would follow.
//!
//! An entry is the host's whole registration, kept whole where it is handed back.

use indexmap::IndexSet;

use super::dedupe::{dedupe_settings_handlers, registered};
use super::event_catalog::supports_handler_type;
use super::gates::passes_gates;
use crate::hooks::contracts::HookSession;
use crate::js::JsValue;

fn text_of<'a>(entry: &'a JsValue, key: &str) -> Option<&'a str> {
    entry.get(key).and_then(JsValue::as_str)
}

pub(super) fn handler_type(entry: &JsValue) -> &str {
    entry
        .get("handler")
        .and_then(|handler| handler.get("type"))
        .and_then(JsValue::as_str)
        .unwrap_or_default()
}

/// Whether the entry passes the session's gates (`passesGates`).
pub(super) fn gated_in(entry: &JsValue, session: &HookSession) -> bool {
    registered(entry).is_some_and(|handler| passes_gates(&handler, session))
}

/// The file names the `FileChanged` matchers register.
#[must_use]
pub fn file_changed_watch_list(entries: &[JsValue], session: &HookSession) -> Vec<String> {
    let names: IndexSet<String> = entries
        .iter()
        .filter(|entry| text_of(entry, "event") == Some("FileChanged"))
        .filter(|entry| supports_handler_type("FileChanged", handler_type(entry)))
        .filter(|entry| gated_in(entry, session))
        .flat_map(|entry| {
            text_of(entry, "matcher")
                .unwrap_or_default()
                .split('|')
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|name| !name.is_empty())
        .collect();
    names.into_iter().collect()
}

/// hooks.md, "CwdChanged" and "FileChanged" output: `watchPaths` "Replaces the current dynamic watch
/// list ... Returning an empty array clears the dynamic list" (`SessionStart` seeds it the same way).
/// The dynamic list after one more output: the output's paths, each once, when the output returned
/// the field (`[]` empties it), else the list as it was. Judgment (spec/judgments.md): an output that
/// names no `watchPaths` leaves the list alone, since only a returned field replaces it.
#[must_use]
pub fn merge_watch_paths(current: Vec<String>, effect: Option<Vec<String>>) -> Vec<String> {
    match effect {
        None => current,
        Some(effect) => effect
            .into_iter()
            .collect::<IndexSet<_>>()
            .into_iter()
            .collect(),
    }
}

/// Whether some registered handler of the event could run in this session: it passes the gates
/// (`disableAllHooks`, trust, plugin enablement, the managed locks) and its type is one the event
/// supports. Judgment (spec/judgments.md): the matcher and `if` are not read, since they need the
/// occurrence the question is asked before; a handler gated off or of an unsupported type never
/// counts, as no run would follow.
#[must_use]
pub fn has_runnable_handler(entries: &[JsValue], session: &HookSession, event: &str) -> bool {
    entries.iter().any(|entry| {
        text_of(entry, "event") == Some(event)
            && supports_handler_type(event, handler_type(entry))
            && gated_in(entry, session)
    })
}

/// The handlers of `entries` that would run in the session, in order: they pass the gates and have a
/// type their event supports (matchers and `if` are not read), and a handler written in several
/// settings files is listed once (hooks.md, "Hook handler fields"). What a host lists in a `/hooks` view.
#[must_use]
pub fn runnable_handlers(entries: Vec<JsValue>, session: &HookSession) -> Vec<JsValue> {
    dedupe_settings_handlers(
        entries
            .into_iter()
            .filter(|entry| {
                let event = text_of(entry, "event").unwrap_or_default();
                supports_handler_type(event, handler_type(entry)) && gated_in(entry, session)
            })
            .collect(),
    )
}

crate::registry::exports! {
    "hooks/select/watch-list" {
        "fileChangedWatchList" => ok |entries: Vec<JsValue>, session: HookSession|
            file_changed_watch_list(&entries, &session);
        "mergeWatchPaths" => ok |current: Vec<String>, effect: Option<Vec<String>>|
            merge_watch_paths(current, effect);
        "hasRunnableHandler" => ok |entries: Vec<JsValue>, session: HookSession, event: String|
            has_runnable_handler(&entries, &session, &event);
        "runnableHandlers" => ok |entries: Vec<JsValue>, session: HookSession|
            runnable_handlers(entries, &session);
    }
}
