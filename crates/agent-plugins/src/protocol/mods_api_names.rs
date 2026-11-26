//! Port of `src/protocol/mods-api-names.ts`, switched.

use indexmap::IndexMap;

use crate::registry::Set;

/// The mods API by namespace and method name as the reference page lists it (plugins/mods/reference.md,
/// "Mods API methods"), in its order.
const MODS_API_REFERENCE: &[(&str, &[&str])] = &[
    ("plugin", &["name", "root"]),
    (
        "ui",
        &[
            "resolve",
            "invalidate",
            "open",
            "close",
            "panes",
            "place",
            "focus",
            "scroll",
            "toast",
            "notify",
            "status",
            "log",
            "notice",
            "ask",
            "copy",
            "selection",
            "blit",
            "view",
        ],
    ),
    ("theme", &["set", "clear"]),
    ("command", &["register", "run", "list"]),
    ("tool", &["register", "call", "check", "list"]),
    ("agent", &["register", "spawn", "list"]),
    ("model", &["complete", "fork", "classify"]),
    ("prompt", &["submit", "read", "fill", "suggest", "compose"]),
    ("turn", &["abort"]),
    (
        "session",
        &[
            "messages",
            "cwd",
            "root",
            "model",
            "turns",
            "id",
            "repo",
            "surfaces",
            "usage",
            "version",
            "compact",
            "send",
            "append",
            "authorize",
        ],
    ),
    ("config", &["list", "set"]),
    ("settings", &["read"]),
    ("env", &["get", "set"]),
    (
        "fs",
        &["read", "write", "list", "exists", "stat", "ancestors"],
    ),
    ("store", &["get", "set", "delete", "keys"]),
    ("state", &["get", "set"]),
    ("clock", &["now", "sleep", "after", "every"]),
    ("http", &["fetch"]),
    ("process", &["run", "spawn"]),
    ("mcp", &["call", "connect"]),
    ("audio", &["play", "speak"]),
    ("telemetry", &["log", "mark"]),
];

/// The members of `$` the declarations name and the reference page does not (AGENTS.md, E2a: the
/// declarations win): `$.ui.cancelNotify` (claude-code/index.d.ts:2536), `$.prompt.media` (:3143) and
/// `$.prompt.paint` (:3168), `$.session.surface` (:2980) and `$.session.identity` (:3081).
const DECLARED_API_MEMBERS: &[(&str, &[&str])] = &[
    ("ui", &["cancelNotify"]),
    ("prompt", &["media", "paint"]),
    ("session", &["surface", "identity"]),
];

/// The mods API as the reference page lists it, in its order.
#[must_use]
pub fn mods_api_reference() -> IndexMap<&'static str, Vec<&'static str>> {
    MODS_API_REFERENCE
        .iter()
        .map(|(name, methods)| (*name, methods.to_vec()))
        .collect()
}

/// The members of `$` the declarations name and the reference page does not.
#[must_use]
pub fn declared_api_members() -> IndexMap<&'static str, Vec<&'static str>> {
    DECLARED_API_MEMBERS
        .iter()
        .map(|(name, methods)| (*name, methods.to_vec()))
        .collect()
}

/// The mods API by namespace and method name: the `$` argument every hook receives, the reference's
/// members and then the declarations' others. Every method is also an event named
/// `<namespace>.<method>` (reference.md, "Mods API calls"), so this table is where the names of those
/// events come from.
#[must_use]
pub fn mods_api() -> IndexMap<&'static str, Vec<&'static str>> {
    let reference = mods_api_reference();
    let declared = declared_api_members();
    let mut api = reference.clone();
    for namespace in ["ui", "prompt", "session"] {
        let methods = [reference.get(namespace), declared.get(namespace)]
            .into_iter()
            .flatten()
            .flatten()
            .copied()
            .collect();
        api.insert(namespace, methods);
    }
    api
}

/// The `status` of each entry `$.agent.list()` returns (reference.md, "Mods API methods"); `idle`
/// and `waiting` require Claude Code v2.1.289 or later, and the kit implements the current behaviour.
pub const AGENT_STATUSES: &[&str] = &[
    "pending",
    "running",
    "waiting",
    "idle",
    "completed",
    "failed",
    "killed",
];

/// The calls whose time is the calling hook's own: "A `$` call's time never counts against the calling
/// hook's budget" (claude-code/index.d.ts:6589), but `$.clock.sleep`'s wait "is the hook's own time and
/// its budget runs on through it, as through no other `$` call" (:3641).
pub const OWN_TIME_CALLS: &[&str] = &["clock.sleep"];

/// The calls declared to answer nothing (`=> void`, claude-code/index.d.ts:2400 `notice`, :2416
/// `invalidate`, :2470 `log`, :2503 `toast`, :2550 `status`): each returns at once and its event goes
/// on through the chain on its own.
pub const FIRED_CALLS: &[&str] = &[
    "ui.notice",
    "ui.invalidate",
    "ui.log",
    "ui.toast",
    "ui.status",
];

/// The calls that answer at once, not with a promise: `$.ui.resolve`, the timers and the fired calls.
/// A misused one throws where it is called.
#[must_use]
pub fn at_once_calls() -> Vec<&'static str> {
    ["ui.resolve", "clock.after", "clock.every"]
        .into_iter()
        .chain(FIRED_CALLS.iter().copied())
        .collect()
}

crate::registry::exports! {
    "protocol/mods-api-names" {
        "MODS_API_REFERENCE" => ok mods_api_reference;
        "DECLARED_API_MEMBERS" => ok declared_api_members;
        "MODS_API" => ok mods_api;
        "AGENT_STATUSES" => ok || AGENT_STATUSES;
        "OWN_TIME_CALLS" => ok || Set { members: OWN_TIME_CALLS.to_vec() };
        "FIRED_CALLS" => ok || FIRED_CALLS;
        "AT_ONCE_CALLS" => ok || Set { members: at_once_calls() };
    }
}
