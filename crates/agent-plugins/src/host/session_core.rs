//! Port of `src/host/session-core.ts`, switched.
//!
//! The state of a session that must outlive a turn, made once by `openSession` and shared by every
//! view `withTurn` makes: the hooks registered so far, with the skills' (a `once` hook that ran is
//! gone for every later turn), what each prefix of `addHooks` registered, the dynamic watch list,
//! the background deliveries and the repeats of an async `SessionStart`. It is one handle the host
//! holds (docs/RUST.md, "The host session"), and the core tells its parts apart by numbers it hands
//! out, never by the identity of a host object: each registered handler has a serial, each
//! registration a serial, and the state is the facts the edge hands over when it takes one up.
//!
//! What stays the host's (the state cell and the state object last taken up, the engine, the
//! skill-hook and monitor ledgers, the listeners and the promises) is kept by the TypeScript edge
//! in `src/host/session-core.ts`.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::session_registry::initial_registry;
use super::session_start_repeats::SessionStartRepeats;
use crate::hooks::contracts::RegisteredHandler as HandlerFacts;
use crate::hooks::register::HookGroup;
use crate::js::JsValue;
use crate::registry::{Handle, Own};

/// A hooks file in the shape of the `hooks` key of settings.
pub type HooksSettings = IndexMap<String, Vec<HookGroup>>;

/// One plugin that runs, as the registry reads it: its full id, its manifest name and its hooks.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveHooks {
    /// The `name@marketplace` id.
    pub id: String,
    /// The manifest name.
    pub name: String,
    /// Its hooks file.
    #[serde(default)]
    pub hooks: HooksSettings,
}

/// The `hooks` of one settings file.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ScopedHooks {
    /// The scope of the file (`PolicyScope`).
    pub scope: String,
    /// Its hooks.
    #[serde(default)]
    pub hooks: HooksSettings,
}

/// The part of the hook gates the registry reads (`HookSessionGates`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeenGates {
    /// The `hooks` of each settings file that declares any, highest precedence first.
    #[serde(default)]
    pub settings_hooks: Vec<ScopedHooks>,
    /// `strictPluginOnlyCustomization` locks the `hooks` surface.
    #[serde(default)]
    pub plugin_only_hooks: bool,
}

/// The state a session last took up, as the facts the registry reads (`Seen`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seen {
    /// The plugins that run, in the order given.
    #[serde(default)]
    pub active: Vec<ActiveHooks>,
    /// The gates its settings give.
    #[serde(default)]
    pub gates: SeenGates,
}

/// A handler the registry holds, under the serial it was registered with.
#[derive(Clone, Debug)]
pub struct Held {
    /// Its identity: a hook registered again under the same id is a new serial.
    pub serial: u32,
    /// `RegisteredHandler.id`.
    pub id: String,
    /// The `RegisteredHandler`, as the host reads it.
    pub entry: JsValue,
    /// Its source and plugin, as the rules read them.
    pub facts: HandlerFacts,
}

/// What one prefix registered: the ids to remove, under the serial of that registration.
#[derive(Clone, Debug)]
pub struct Registration {
    /// Which registration of the prefix this is (`===` in the TypeScript).
    pub serial: u32,
    /// The ids it registered.
    pub ids: Vec<String>,
}

/// The mutable part of a session's core.
#[derive(Debug, Default)]
pub struct CoreState {
    /// The state and gates the registry was last made for.
    pub(crate) seen: Seen,
    /// The registered handlers, in registry order.
    pub(crate) registry: Vec<Held>,
    /// Their entries, as the selection rules read them.
    pub(crate) values: Vec<JsValue>,
    /// What each prefix registered through `addHooks` or a skill, by prefix.
    pub(crate) registrations: IndexMap<String, Registration>,
    /// The dynamic part of the FileChanged watch list: the last `watchPaths` an output returned.
    pub(crate) watch_paths: Vec<String>,
    pub(crate) next_serial: u32,
    pub(crate) revision: u32,
    pub(crate) answered: Option<u32>,
    pub(crate) sent: BTreeSet<u32>,
}

impl CoreState {
    /// The next serial, never given before in this session.
    pub const fn serial(&mut self) -> u32 {
        let serial = self.next_serial;
        self.next_serial = self.next_serial.wrapping_add(1);
        serial
    }

    /// `entries` as held handlers, each under a new serial; an entry that is not a handler is left
    /// out (none is: they come from `registerHooks`).
    pub fn hold(&mut self, entries: Vec<JsValue>) -> Vec<Held> {
        entries
            .into_iter()
            .filter_map(|entry| {
                let facts = HandlerFacts::deserialize(entry.clone()).ok()?;
                let id = entry.get("id").and_then(JsValue::as_str)?.to_owned();
                Some(Held {
                    serial: self.serial(),
                    id,
                    entry,
                    facts,
                })
            })
            .collect()
    }

    /// Replaces the registry; the mirror of the host's reads it again.
    pub fn set_registry(&mut self, registry: Vec<Held>) {
        self.values = registry.iter().map(|held| held.entry.clone()).collect();
        self.registry = registry;
        self.revision = self.revision.wrapping_add(1);
    }

    /// The registry's entries, as the host reads them.
    pub fn entry_values(&self) -> &[JsValue] {
        &self.values
    }
}

/// The core of one session.
#[derive(Debug, Default)]
pub struct SessionCore {
    pub(crate) state: Mutex<CoreState>,
    /// What each async `SessionStart` hook last gave, so a repeat is not handed on again.
    pub(crate) session_starts: SessionStartRepeats,
}

impl SessionCore {
    /// Its state, for one operation.
    pub fn lock(&self) -> MutexGuard<'_, CoreState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

crate::registry::methods! {
    SessionCore {}
}

/// The registry as the host's mirror reads it: the serials in registry order, and the entries of
/// those it was not sent before (an entry never changes under its serial).
#[derive(Debug, Serialize)]
pub struct Mirror {
    serials: Vec<u32>,
    fresh: Vec<(u32, JsValue)>,
}

/// The core of a session opened over `seen`: the registry its state declares.
#[must_use]
pub fn create_session_core(seen: Seen) -> SessionCore {
    let core = SessionCore::default();
    {
        let mut state = core.lock();
        let declared = initial_registry(&seen);
        let registry = state.hold(declared);
        state.set_registry(registry);
        state.seen = seen;
    }
    core
}

/// The registry since the host last read it; `None` when it has not changed.
pub fn entries(core: &SessionCore) -> Option<Mirror> {
    let mut state = core.lock();
    if state.answered == Some(state.revision) {
        return None;
    }
    state.answered = Some(state.revision);
    let serials: Vec<u32> = state.registry.iter().map(|held| held.serial).collect();
    let fresh = state
        .registry
        .iter()
        .filter(|held| !state.sent.contains(&held.serial))
        .map(|held| (held.serial, held.entry.clone()))
        .collect();
    state.sent = serials.iter().copied().collect();
    Some(Mirror { serials, fresh })
}

/// Brings a session over to a new state (docs/API.md, "Updating"): the handlers of kind `plugin` and
/// `settings` are rebuilt from it; those a session registered since it opened (skill, agent,
/// session) stay unless their plugin no longer runs or the new plugin-only lock silences them; the
/// registrations follow what stayed. A consumed `once` key stays consumed (its handler is not in
/// the registry to come back), and the dynamic watch list is kept. The edge calls it once per state
/// object it has not taken up (`currentOf`), and settles the skill-hook ledger itself.
pub fn take_up(core: &SessionCore, seen: Seen) {
    let mut state = core.lock();
    super::session_registry::refresh_registry(&mut state, &seen);
    state.seen = seen;
}

crate::registry::exports! {
    "host/session-core" {
        "createSessionCore" => ok |seen: Seen| Handle::own(create_session_core(seen));
        "takeUp" => ok |core: Own<SessionCore>, seen: Seen| take_up(&core, seen);
        "entries" => nullable |core: Own<SessionCore>| entries(&core);
    }
}

/// The core shared with a method's handle (a registration's `remove`).
pub type SharedCore = Arc<SessionCore>;
