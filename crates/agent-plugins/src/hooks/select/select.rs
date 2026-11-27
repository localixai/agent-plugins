//! Port of `src/hooks/select/select.ts`, switched.
//!
//! The registered handlers are the host's own objects, which its callers tell apart by identity
//! (the `monitor` map is keyed by entry): the core reads copies and answers with each selected
//! entry's place in the registry, and the wrapper hands back the host's entries at those places.

use std::borrow::Borrow;

use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::hooks::contracts::{HookSession, MonitorTreatment, ToolCallView};
use crate::js::{JsValue, string_of};
use crate::registry::{Callback, Port};

use super::dedupe::dedupe_settings_handlers;
use super::event_catalog::{fires_for_tool, supports_handler_type};
use super::if_guard::{GuardSubject, guard_with_if, if_of};
use super::matcher::matcher_selects;
use super::matcher_targets::call_of;
use super::mcp_client::{lacks_mcp_client, no_mcp_client_message};
use super::watch_list::{gated_in, handler_type};

/// What a selection is asked: the event, its facts, the session, the reader of a handler's `if`
/// (without one a handler with `if` runs and a warning says why) and the catalog that names the
/// canonical model of a model switch (without one every model-switch hook runs).
pub struct SelectRequest {
    /// The event.
    pub event: String,
    /// Its facts.
    pub facts: JsValue,
    /// The session.
    pub session: HookSession,
    /// Reads a handler's `if`.
    pub if_evaluator: Option<Callback>,
    /// Names the canonical model of a model switch.
    pub model_catalog: Option<Port>,
}

/// The handlers an event runs, by their place in the registry, with the issues found and how each
/// Monitor call reached its handler (`Selection`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Selection {
    /// The selected entries' places in the registry.
    pub selected: Vec<usize>,
    /// The issues found.
    pub issues: Vec<PluginIssue>,
    /// Only for a Monitor call on `PreToolUse` or `PermissionRequest`: an entry's place and its
    /// treatment.
    pub monitor: Vec<(usize, MonitorTreatment)>,
}

struct Verdict {
    selected: bool,
    monitor: Option<MonitorTreatment>,
    issues: Vec<PluginIssue>,
}

const SKIPPED: Verdict = Verdict {
    selected: false,
    monitor: None,
    issues: Vec::new(),
};

/// An entry of the event, with its place in the registry and among the event's entries.
struct Indexed<'a> {
    index: usize,
    position: usize,
    entry: &'a JsValue,
}

impl Borrow<JsValue> for Indexed<'_> {
    fn borrow(&self) -> &JsValue {
        self.entry
    }
}

/// Where an issue about one handler is reported: the event and the handler's place under it.
fn path_of(event: &str, index: usize) -> String {
    format!("hooks.{event}[{index}]")
}

/// One handler against the event: its type is supported (an unsupported one "is skipped",
/// reported as a warning: spec/judgments.md), its matcher selects the occurrence, an `mcp_tool` one
/// has an MCP client to call, and its `if` lets it run.
async fn judge(
    entry: &JsValue,
    index: usize,
    request: &SelectRequest,
    call: Option<&ToolCallView>,
) -> Verdict {
    let event = request.event.as_str();
    let path = path_of(event, index);
    let kind = handler_type(entry);
    if !supports_handler_type(event, kind) {
        let given = entry
            .get("handler")
            .and_then(|handler| handler.get("type"))
            .map_or_else(|| "undefined".to_owned(), string_of);
        let message = format!("A hook of type {given} is not supported on {event} and is skipped");
        return Verdict {
            issues: vec![warning(&path, &message)],
            ..SKIPPED
        };
    }
    let catalog = request.model_catalog.as_ref();
    let matched = matcher_selects(event, &request.facts, entry, catalog).await;
    if !matched.selects {
        return SKIPPED;
    }
    if lacks_mcp_client(event, &request.facts, kind) {
        return Verdict {
            issues: vec![warning(&path, &no_mcp_client_message(event))],
            ..SKIPPED
        };
    }
    let subject = GuardSubject {
        call: call.cloned(),
        monitor: matched.monitor,
        evaluate: request.if_evaluator,
    };
    let guard = guard_with_if(if_of(entry), &subject, &path).await;
    Verdict {
        selected: guard.runs,
        issues: guard.issues,
        monitor: matched.monitor,
    }
}

/// The handlers an event runs (hooks.md, "How a hook resolves"): the registered handlers of the
/// event that nothing gates off (`disableAllHooks`, workspace trust, a disabled plugin), with a
/// settings handler defined twice kept once, whose type the event supports, whose matcher selects
/// the occurrence and whose `if` lets them run. Tool calls an event skips select nothing.
pub async fn select_handlers(entries: &[JsValue], request: &SelectRequest) -> Selection {
    let event = request.event.as_str();
    let of_event: Vec<(usize, &JsValue)> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.get("event").and_then(JsValue::as_str) == Some(event))
        .collect();
    let call = call_of(event, &request.facts);
    if call
        .as_ref()
        .is_some_and(|call| !fires_for_tool(event, &call.tool_name))
    {
        return Selection::default();
    }
    let gated: Vec<Indexed<'_>> = of_event
        .iter()
        .enumerate()
        .filter(|(_, (_, entry))| gated_in(entry, &request.session))
        .map(|(position, (index, entry))| Indexed {
            index: *index,
            position,
            entry,
        })
        .collect();
    let mut selection = Selection::default();
    for item in dedupe_settings_handlers(gated) {
        let verdict = judge(item.entry, item.position, request, call.as_ref()).await;
        selection.issues.extend(verdict.issues);
        if !verdict.selected {
            continue;
        }
        selection.selected.push(item.index);
        if let Some(monitor) = verdict.monitor {
            selection.monitor.push((item.index, monitor));
        }
    }
    selection
}

crate::registry::exports! {
    "hooks/select/select" {
        "selectHandlers" => async ok |entries: Vec<JsValue>, event: String, facts: JsValue,
            session: HookSession, if_evaluator: Option<Callback>, model_catalog: Option<Port>|
            select_handlers(&entries, &SelectRequest {
                event,
                facts,
                session,
                if_evaluator,
                model_catalog,
            })
            .await;
    }
}
