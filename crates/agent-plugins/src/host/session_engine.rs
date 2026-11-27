//! Port of `src/host/session-engine.ts`, switched.
//!
//! The dispatch of a session (docs/API.md, "Hooks"): the session's engine over the session's own
//! registry and `once` claims, which the core holds by serial, so nothing of the registry crosses.
//! The engine itself is built by the edge, again only when what it is built over changes (the
//! host's state, the turn's ports, working directory, added folders or trust), as it holds the
//! host's ports; a `Stop` that core handlers share is dispatched without the Stop cap, which the
//! session then applies once to the combined outcome (Agent Plugins Extended §18.1.2, J18.9).

use std::sync::Arc;

use serde::Deserialize;

use crate::hooks::contracts::HookSourceKind;
use crate::hooks::dispatch::config::DispatchRequest;
use crate::hooks::dispatch::dispatch_event;
use crate::hooks::engine::HookEngine;
use crate::hooks::select::once::Claims;
use crate::js::{JsObject, JsValue};
use crate::registry::Own;

use super::session_core::SessionCore;

/// Which of the session's handlers one dispatch reaches, and whether the Stop cap counts it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchScope {
    /// Only the handlers of these sources, when given.
    #[serde(default)]
    pub sources_only: Option<Vec<HookSourceKind>>,
    /// Whether the Stop cap counts this dispatch.
    pub capped: bool,
}

/// One dispatch of the session's registry through `engine`, within `scope`.
///
/// # Errors
///
/// The facts are not an input the event's schema reads.
pub async fn dispatch_in(
    engine: &HookEngine,
    core: &Arc<SessionCore>,
    request: DispatchRequest,
    scope: DispatchScope,
) -> Result<JsObject, String> {
    let sources_only = scope.sources_only.as_deref();
    let entries: Vec<(u32, JsValue)> = core
        .lock()
        .registry
        .iter()
        .filter(|held| sources_only.is_none_or(|kinds| kinds.contains(&held.facts.source.kind)))
        .map(|held| (held.serial, held.entry.clone()))
        .collect();
    let claims: Arc<dyn Claims> = Arc::clone(core) as Arc<dyn Claims>;
    dispatch_event(entries, request, engine.config(), &claims, scope.capped).await
}

crate::registry::exports! {
    "host/session-engine" {
        "dispatchIn" => async fallible |engine: Own<HookEngine>, core: Own<SessionCore>,
            request: DispatchRequest, scope: DispatchScope|
            dispatch_in(&engine, &core.0, request, scope).await;
    }
}
