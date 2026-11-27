//! Port of `src/host/session-hook-scope.ts`, switched.
//!
//! The scoped-hook part of one turn's view over the session's core: `addHooks`, `removeHooks`,
//! `runScoped`'s turn scope, `hasHooks`, `hookEntries`, `fileChangedWatchList` and what a dispatch
//! leaves in the core. The view's `session` is read by the edge at each use and passed in, so the
//! gates it carries are the current settings'.

use std::sync::{Mutex, PoisonError};

use indexmap::IndexSet;
use serde::Deserialize;

use super::session_core::{HooksSettings, SessionCore};
use super::session_registry::{Registered, drop_registration, register_entries};
use super::session_start_repeats::{Watched, WatchedOutcome};
use crate::hooks::contracts::HookSession;
use crate::hooks::register::{HookDeclaration, register_hooks};
use crate::hooks::select::watch_list::{
    file_changed_watch_list, has_runnable_handler, merge_watch_paths, runnable_handlers,
};
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::registry::{Handle, HostPromise, Own};

/// How long hooks added with `addHooks` live: the session, or until the `runScoped` turn ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookLifetime {
    /// Until the session ends or they are removed.
    #[default]
    Session,
    /// Until the `runScoped` turn ends.
    Turn,
}

/// The options of `addHooks`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddHooksOptions {
    /// Default `session`. `turn` needs the view `runScoped` gives, which drops the hooks when it ends.
    #[serde(default)]
    pub lifetime: Option<HookLifetime>,
    /// The full `name@marketplace` id of the plugin a skill or agent hook belongs to.
    #[serde(default)]
    pub plugin_id: Option<String>,
}

#[derive(Debug, Default)]
struct TurnState {
    handles: Vec<Registered>,
    ended: bool,
}

/// The turn-lifetime registrations of one `runScoped`, dropped together when it ends.
#[derive(Debug, Default)]
pub struct TurnScope {
    state: Mutex<TurnState>,
}

impl TurnScope {
    /// Ends the scope: every registration it holds is removed, and it registers nothing more.
    pub fn end(&self) {
        let handles = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.ended = true;
            std::mem::take(&mut state.handles)
        };
        handles.iter().for_each(Registered::remove);
    }
}

crate::registry::methods! {
    TurnScope {
        "end" => ok |this| this.end();
    }
}

/// A turn scope for one `runScoped`.
#[must_use]
pub fn create_turn_scope() -> TurnScope {
    TurnScope::default()
}

/// The part of a dispatch's outcome the core keeps: the effects' `watchPaths`, and what the repeat
/// check of an async `SessionStart` reads.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedOutcome {
    event: String,
    #[serde(default)]
    background: Vec<HostPromise<crate::js::JsObject>>,
    #[serde(default)]
    additional_context: Vec<JsValue>,
    #[serde(default)]
    system_messages: Vec<JsValue>,
    #[serde(default)]
    effects: RecordedEffects,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordedEffects {
    #[serde(default)]
    watch_paths: Option<Vec<String>>,
}

/// What a dispatch leaves in the core: `watchPaths` of a SessionStart, CwdChanged or FileChanged
/// output replaces the dynamic watch list (no other event's output carries the field), and the
/// background promises are answered to join the queue `takeBackground` reads, an async
/// `SessionStart` hook's repeat emptied (`source` is the `SessionStart` input's). The edge drops the
/// `once` registrations its runs spent, and again as each background promise settles.
pub fn record_outcome(
    core: &SessionCore,
    outcome: RecordedOutcome,
    source: &JsValue,
) -> Vec<Watched> {
    {
        let mut state = core.lock();
        let current = std::mem::take(&mut state.watch_paths);
        state.watch_paths = merge_watch_paths(current, outcome.effects.watch_paths);
    }
    core.session_starts.watch(
        WatchedOutcome {
            event: outcome.event,
            background: outcome.background,
            additional_context: outcome.additional_context,
            system_messages: outcome.system_messages,
        },
        source,
    )
}

/// Adds the hooks of a run, an agent or a skill under `prefix` (the ids of its entries begin with
/// it; two registrations of one session never share one). Registering a prefix again replaces what
/// it registered. The hooks pass the same gates as any: the source decides them. A `turn` lifetime
/// needs the scope of `session.runScoped`, still running.
pub fn add_hooks(
    core: &Own<SessionCore>,
    prefix: &str,
    hooks: &HooksSettings,
    source: JsValue,
    options: AddHooksOptions,
    scope: Option<&TurnScope>,
) -> Result<Registered, String> {
    let lifetime = options.lifetime.unwrap_or_default();
    if lifetime == HookLifetime::Turn {
        let Some(scope) = scope else {
            return Err(
                "addHooks with lifetime \"turn\" needs the view of session.runScoped".into(),
            );
        };
        if scope
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .ended
        {
            return Err(
                "addHooks with lifetime \"turn\" on a view whose session.runScoped has ended"
                    .into(),
            );
        }
    }
    let entries = register_hooks(
        hooks,
        &HookDeclaration {
            id_prefix: prefix.to_owned(),
            source,
            plugin_id: options.plugin_id,
        },
    );
    let values = entries
        .iter()
        .filter_map(|entry| to_js_value(entry).ok())
        .collect();
    let handle = register_entries(&core.0, prefix, values);
    if let (HookLifetime::Turn, Some(scope)) = (lifetime, scope) {
        scope
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .handles
            .push(handle.again());
    }
    Ok(handle)
}

/// The literal names of the FileChanged matchers, then the dynamic `watchPaths`, each once.
pub fn watch_list(core: &SessionCore, session: &HookSession) -> Vec<String> {
    let state = core.lock();
    let mut names: IndexSet<String> = file_changed_watch_list(state.entry_values(), session)
        .into_iter()
        .collect();
    names.extend(state.watch_paths.iter().cloned());
    names.into_iter().collect()
}

crate::registry::exports! {
    "host/session-hook-scope" {
        "createTurnScope" => ok || Handle::own(create_turn_scope());
        "recordOutcome" => ok |core: Own<SessionCore>, outcome: RecordedOutcome, source: JsValue|
            record_outcome(&core, outcome, &source);
        "addHooks" => fallible |core: Own<SessionCore>, prefix: String, hooks: HooksSettings,
            source: JsValue, options: AddHooksOptions, scope: Option<Own<TurnScope>>|
            add_hooks(&core, &prefix, &hooks, source, options, scope.as_deref()).map(Handle::new);
        "removeHooks" => ok |core: Own<SessionCore>, prefix: String|
            drop_registration(&mut core.lock(), &prefix);
        "hasHooks" => ok |core: Own<SessionCore>, session: HookSession, event: String|
            has_runnable_handler(core.lock().entry_values(), &session, &event);
        "hookEntries" => ok |core: Own<SessionCore>, session: HookSession|
            runnable_handlers(core.lock().entry_values().to_vec(), &session);
        "fileChangedWatchList" => ok |core: Own<SessionCore>, session: HookSession|
            watch_list(&core, &session);
    }
}
