//! Port of `src/protocol/mods-events.ts`, switched.
//!
//! The events a mod can handle (plugins/mods/reference.md, "Events"): those with a name of their
//! own, by group; the settings hook events (`classic.<Event>`) and the mods API calls (`fs.read`…)
//! are events too, and are derived from their own tables.

use indexmap::IndexMap;
use serde::Serialize;

use super::hook_events::HOOK_EVENTS;
use super::mods_api_names::mods_api;

/// The events a mod can handle that have a name of their own, by group (plugins/mods/reference.md,
/// "Events"). The settings hook events (`classic.<Event>`) and the mods API calls (`fs.read`…) are
/// events too, and are derived from their own tables below.
pub const MOD_EVENT_GROUPS: &[(&str, &[&str])] = &[
    ("tools", &["tool.call", "tool.check", "tool.describe"]),
    (
        "prompts",
        &[
            "prompt.submit",
            "prompt.fill",
            "prompt.suggest",
            "prompt.edit",
            "prompt.autocomplete",
            "prompt.compose",
            "prompt.section",
            "prompt.context",
            "prompt.attachment",
            "prompt.mention",
            "skill.prompt",
            "attribution.text",
        ],
    ),
    (
        "commands",
        &[
            "command.run",
            "command.describe",
            "config.set",
            "config.describe",
            "settings.read",
        ],
    ),
    ("turns", &["turn.start", "turn.step", "turn.complete"]),
    (
        "session",
        &[
            "session.start",
            "session.end",
            "session.compact",
            "session.receive",
            "session.send",
            "session.append",
            "session.attach",
            "session.detach",
            "session.measure",
        ],
    ),
    ("subagents", &["agent.offer", "agent.spawn"]),
    (
        "interface",
        &[
            "ui.render",
            "ui.resolve",
            "ui.press",
            "ui.input",
            "ui.select",
            "ui.focus",
            "ui.scroll",
            "ui.close",
            "ui.message",
            "ui.fault",
        ],
    ),
    ("mods", &["plugin.register", "engine.create"]),
    ("telemetry", &["telemetry.log", "telemetry.mark"]),
];

/// `MOD_EVENT_GROUPS` as an object.
#[must_use]
pub fn mod_event_groups() -> IndexMap<&'static str, &'static [&'static str]> {
    MOD_EVENT_GROUPS.iter().copied().collect()
}

/// The 48 documented events, flat, in the order of the reference.
#[must_use]
pub fn mod_event_names() -> Vec<&'static str> {
    MOD_EVENT_GROUPS
        .iter()
        .flat_map(|(_, names)| names.iter().copied())
        .collect()
}

/// The engine's own events the declarations name and the reference page does not (AGENTS.md,
/// E2a: the declarations win): `prompt.change`, raised after the prompt box changed
/// (claude-code/index.d.ts:4416, `PromptChangeInput`).
pub const DECLARED_EVENT_NAMES: &[&str] = &["prompt.change"];

/// Every event of the engine's own with a name of its own: the reference's, then the
/// declarations' others.
#[must_use]
pub fn engine_event_names() -> Vec<&'static str> {
    let mut names = mod_event_names();
    names.extend(DECLARED_EVENT_NAMES);
    names
}

/// The documented events a mod's `$` call fires as the host serves it, not as the engine's own
/// event: they are in the declarations' `OpEventOf` (claude-code/index.d.ts:7491, :7672), not in
/// `EventCalls` (:5011).
const HOST_SERVED: [&str; 2] = ["settings.read", "ui.close"];

/// Whether `name` is one of the engine's own events (`EventCalls`, claude-code/index.d.ts:5011): a
/// mod's call of one "runs the same chain with the calling hook alone skipped", every hook below
/// the caller included. A call the host serves (`OpEventOf`, :7292) reaches only the hooks above
/// the caller.
#[must_use]
pub fn is_engine_own_event(name: &str) -> bool {
    (DECLARED_EVENT_NAMES.contains(&name)
        || MOD_EVENT_GROUPS
            .iter()
            .any(|(_, names)| names.contains(&name)))
        && !HOST_SERVED.contains(&name)
}

/// The event of a settings hook event: `classic.Stop`, `classic.PostToolUse`. `e` is the hook's
/// stdin JSON.
#[must_use]
pub fn classic_event_names() -> Vec<String> {
    HOOK_EVENTS
        .iter()
        .map(|event| format!("classic.{event}"))
        .collect()
}

/// The members of the table that are values, not calls: `$.plugin.name` and `$.plugin.root` are
/// "this plugin's name and directory" (reference.md, "Mods API methods"). A value fires no event,
/// so these are not events (judgment: the table lists them under "Methods", and the events are
/// "every mods API method"). `$.prompt.media` is "The engine's own read, no event: no hook answers
/// for it" (claude-code/index.d.ts:3137), so it is no event either.
const API_VALUES: [&str; 3] = ["plugin.name", "plugin.root", "prompt.media"];

/// The event of a mods API call: `fs.read`, `model.complete`, `ui.open`.
#[must_use]
pub fn api_event_names() -> Vec<String> {
    mods_api()
        .iter()
        .flat_map(|(namespace, methods)| {
            methods
                .iter()
                .map(move |method| format!("{namespace}.{method}"))
        })
        .filter(|name| !API_VALUES.contains(&name.as_str()))
        .collect()
}

/// The wildcards `on` takes in place of an event name (events.md, "Filter which events a hook
/// handles").
pub const WILDCARDS: [&str; 2] = ["classic.*", "*"];

/// The events a hook handles as an async generator, because the event streams (reference.md,
/// "Events": hooks on `turn.step` and `process.spawn` are async generators; every other hook is an
/// async function).
pub const STREAM_EVENTS: [&str; 2] = ["turn.step", "process.spawn"];

/// The telemetry events. `*` does not match them; a hook on one takes its own name and the filter
/// `{ to: 'collector' }` (reference.md, "Telemetry").
pub const TELEMETRY_EVENTS: [&str; 2] = ["telemetry.log", "telemetry.mark"];

/// The filter of a telemetry hook.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct TelemetryFilter {
    /// `collector`.
    pub to: &'static str,
}

/// `{ to: 'collector' }`.
pub const TELEMETRY_FILTER: TelemetryFilter = TelemetryFilter { to: "collector" };

/// `{ plugin, tier }` of whoever fired an event. Claude Code itself is `{ plugin: 'engine', tier:
/// 'core' }`.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct HookOrigin {
    /// The plugin.
    pub plugin: &'static str,
    /// Its tier (`MOD_TIERS`).
    pub tier: &'static str,
}

/// Claude Code itself.
pub const ENGINE_ORIGIN: HookOrigin = HookOrigin {
    plugin: "engine",
    tier: "core",
};

/// `HookFailure.kind` (claude-code/index.d.ts:5448): a hook threw or misreturned, or ran over its
/// time.
pub const HOOK_FAILURE_KINDS: [&str; 2] = ["throw", "timeout"];

crate::registry::exports! {
    "protocol/mods-events" {
        "MOD_EVENT_GROUPS" => ok mod_event_groups;
        "MOD_EVENT_NAMES" => ok mod_event_names;
        "DECLARED_EVENT_NAMES" => ok || DECLARED_EVENT_NAMES;
        "ENGINE_EVENT_NAMES" => ok engine_event_names;
        "isEngineOwnEvent" => ok |name: String| is_engine_own_event(&name);
        "CLASSIC_EVENT_NAMES" => ok classic_event_names;
        "API_EVENT_NAMES" => ok api_event_names;
        "WILDCARDS" => ok || WILDCARDS;
        "STREAM_EVENTS" => ok || STREAM_EVENTS;
        "TELEMETRY_EVENTS" => ok || TELEMETRY_EVENTS;
        "TELEMETRY_FILTER" => ok || TELEMETRY_FILTER;
        "ENGINE_ORIGIN" => ok || ENGINE_ORIGIN;
        "HOOK_FAILURE_KINDS" => ok || HOOK_FAILURE_KINDS;
    }
}

#[cfg(test)]
mod tests {
    use super::{engine_event_names, is_engine_own_event, mod_event_names};

    #[test]
    fn forty_eight_documented_events_and_the_declared_one() {
        assert_eq!(mod_event_names().len(), 48);
        assert_eq!(engine_event_names().last(), Some(&"prompt.change"));
        assert!(is_engine_own_event("prompt.change"));
        assert!(!is_engine_own_event("ui.close"));
        assert!(!is_engine_own_event("fs.read"));
    }
}
