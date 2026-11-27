//! Port of `src/host/session-registry.ts`, switched.
//!
//! The hooks of a session and every change made to them: what the state declares, what a skill, an
//! agent or the host's run registers, what an update keeps and what a `once` run spends. Nothing
//! else writes the core's registry.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::session_core::{
    ActiveHooks, CoreState, Held, Registration, Seen, SeenGates, SessionCore, SharedCore,
};
use crate::hooks::register::{HookDeclaration, register_hooks};
use crate::hooks::select::lock_gates::plugin_only_allows;
use crate::hooks::source_kind::policy_of;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::registry::{Handle, Own};

/// Where the hooks of a settings file of each scope are declared (hooks.md, "Hook locations"): the
/// scope, then the source's `scope` and whether it is `managed`.
const SETTINGS_SOURCE: [(&str, Option<&str>, bool); 7] = [
    ("managed", None, true),
    ("parent", None, false),
    ("flag", None, false),
    ("local", Some("local"), false),
    ("project", Some("project"), false),
    ("user", Some("user"), false),
    ("addDir", None, false),
];

/// The `HookSource` of a settings file of `scope`.
fn settings_source(scope: &str) -> JsValue {
    let (scope_field, managed) = SETTINGS_SOURCE
        .iter()
        .find(|(name, _, _)| *name == scope)
        .map_or((None, false), |(_, field, managed)| (*field, *managed));
    let mut members = JsObject::new();
    members.insert("kind".into(), JsValue::from("settings"));
    members.insert("label".into(), JsValue::from("[settings]"));
    if managed {
        members.insert("managed".into(), JsValue::from(true));
    }
    if let Some(field) = scope_field {
        members.insert("scope".into(), JsValue::from(field));
    }
    JsValue::from(members)
}

fn values(entries: &[crate::hooks::register::RegisteredHandler]) -> Vec<JsValue> {
    entries
        .iter()
        .filter_map(|entry| to_js_value(entry).ok())
        .collect()
}

fn plugin_entries(plugin: &ActiveHooks) -> Vec<JsValue> {
    let source = JsValue::from([
        ("kind", JsValue::from("plugin")),
        ("label", JsValue::from(format!("[plugin:{}]", plugin.name))),
        ("pluginName", JsValue::from(plugin.name.as_str())),
    ]);
    values(&register_hooks(
        &plugin.hooks,
        &HookDeclaration {
            id_prefix: plugin.id.clone(),
            source,
            plugin_id: Some(plugin.id.clone()),
        },
    ))
}

fn settings_entries(gates: &SeenGates) -> Vec<JsValue> {
    gates
        .settings_hooks
        .iter()
        .flat_map(|scoped| {
            values(&register_hooks(
                &scoped.hooks,
                &HookDeclaration {
                    id_prefix: format!("settings:{}", scoped.scope),
                    source: settings_source(&scoped.scope),
                    plugin_id: None,
                },
            ))
        })
        .collect()
}

/// The hooks the state declares itself, in registry order (Agent Plugins Extended §7.1, Judgment
/// J7.2): the settings files' first, highest precedence first (§12.3), then every enabled
/// plugin's in load order; skill and agent frontmatter hooks follow as they register. The order
/// fixes the merge of several hooks' scalar fields and the order of their messages.
fn declared_entries(seen: &Seen) -> Vec<JsValue> {
    let mut entries = settings_entries(&seen.gates);
    entries.extend(seen.active.iter().flat_map(plugin_entries));
    entries
}

/// The registry a session opens with.
#[must_use]
pub fn initial_registry(seen: &Seen) -> Vec<JsValue> {
    declared_entries(seen)
}

/// A handler the session registered after it opened: an update keeps it (`SourcePolicy.scoped`).
const fn is_scoped(held: &Held) -> bool {
    policy_of(held.facts.source.kind).scoped
}

/// Whether a scoped handler outlives an update: the plugin it belongs to still runs, and the
/// plugin-only lock of the new settings does not silence its source (judgment, spec/judgments.md: a
/// handler a lock silences is dropped at any update that finds the lock set).
fn survives(held: &Held, seen: &Seen) -> bool {
    let plugin_runs = held
        .facts
        .plugin_id
        .as_ref()
        .is_none_or(|id| seen.active.iter().any(|plugin| &plugin.id == id));
    plugin_runs && (!seen.gates.plugin_only_hooks || plugin_only_allows(&held.facts))
}

/// The registrations without the ids an update dropped; one left with none is forgotten.
fn settle_registrations(state: &mut CoreState, lost: &BTreeSet<String>) {
    state.registrations.retain(|_, registration| {
        registration.ids.retain(|id| !lost.contains(id));
        !registration.ids.is_empty()
    });
}

/// The registry of a new state: the declared handlers are rebuilt from it, the scoped ones stay
/// when they survive it (`survives`), and the registrations follow what stayed.
pub fn refresh_registry(state: &mut CoreState, seen: &Seen) {
    let (kept, gone): (Vec<Held>, Vec<Held>) = state
        .registry
        .iter()
        .filter(|held| is_scoped(held))
        .cloned()
        .partition(|held| survives(held, seen));
    let lost: BTreeSet<String> = gone.into_iter().map(|held| held.id).collect();
    let mut registry = state.hold(declared_entries(seen));
    registry.extend(kept);
    state.set_registry(registry);
    settle_registrations(state, &lost);
}

/// The registry without the handlers whose id is in `ids`; unchanged when none is.
fn without(state: &mut CoreState, ids: &[String]) {
    if ids.is_empty() {
        return;
    }
    let registry: Vec<Held> = state
        .registry
        .iter()
        .filter(|held| !ids.contains(&held.id))
        .cloned()
        .collect();
    if registry.len() != state.registry.len() {
        state.set_registry(registry);
    }
}

/// Drops what `prefix` registered: by the ids it registered, so removal never matches by text.
pub fn drop_registration(state: &mut CoreState, prefix: &str) {
    let ids = state
        .registrations
        .shift_remove(prefix)
        .map(|registration| registration.ids)
        .unwrap_or_default();
    without(state, &ids);
}

/// What a registration's handle removes: the prefix, while it still holds this registration.
#[derive(Debug)]
pub struct Registered {
    core: SharedCore,
    prefix: String,
    serial: u32,
}

impl Registered {
    /// Removes the registration; it does nothing once the prefix was registered again or removed.
    pub fn remove(&self) {
        let mut state = self.core.lock();
        let holds = state
            .registrations
            .get(&self.prefix)
            .is_some_and(|registration| registration.serial == self.serial);
        if holds {
            drop_registration(&mut state, &self.prefix);
        }
    }

    /// The same registration, for a turn scope to remove when it ends.
    #[must_use]
    pub fn again(&self) -> Self {
        Self {
            core: Arc::clone(&self.core),
            prefix: self.prefix.clone(),
            serial: self.serial,
        }
    }
}

crate::registry::methods! {
    Registered {
        "remove" => ok |this| this.remove();
    }
}

/// Registers `entries` under `prefix`, replacing the ones the prefix registered before ("replaces,
/// never stacks", as a skill invoked again does). An entry the current state would drop at an
/// update (its plugin no longer runs, or the plugin-only lock silences it) is not registered, as
/// docs/API.md ("Updating") keeps scoped hooks "unless their pluginId no longer runs". The handle
/// removes them unless the prefix was registered again since.
pub fn register_entries(core: &SharedCore, prefix: &str, entries: Vec<JsValue>) -> Registered {
    let mut state = core.lock();
    let held = state.hold(entries);
    let kept: Vec<Held> = held
        .into_iter()
        .filter(|held| survives(held, &state.seen))
        .collect();
    let serial = state.serial();
    let registration = Registration {
        serial,
        ids: kept.iter().map(|held| held.id.clone()).collect(),
    };
    let previous = state.registrations.insert(prefix.to_owned(), registration);
    without(
        &mut state,
        &previous
            .map(|registration| registration.ids)
            .unwrap_or_default(),
    );
    if !kept.is_empty() {
        let mut registry = state.registry.clone();
        registry.extend(kept);
        state.set_registry(registry);
    }
    Registered {
        core: Arc::clone(core),
        prefix: prefix.to_owned(),
        serial,
    }
}

/// Drops the `once` registrations a run spent (`OnceClaims`): by serial, so a hook registered
/// again under the same id while the run was in flight stays.
pub fn drop_spent(state: &mut CoreState, spent: &[u32]) {
    let registry: Vec<Held> = state
        .registry
        .iter()
        .filter(|held| !spent.contains(&held.serial))
        .cloned()
        .collect();
    if registry.len() != state.registry.len() {
        state.set_registry(registry);
    }
}

crate::registry::exports! {
    "host/session-registry" {
        "registerEntries" => ok |core: Own<SessionCore>, prefix: String, entries: Vec<JsValue>|
            Handle::new(register_entries(&core.0, &prefix, entries));
    }
}
