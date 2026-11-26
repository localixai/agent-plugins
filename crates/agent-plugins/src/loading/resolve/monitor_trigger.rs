//! Port of `src/loading/resolve/monitor-trigger.ts`, switched.
//!
//! What can start a monitor (plugins/manifest-reference.md, "monitors", the `when` row): a session
//! starting, plugins reloading, or a skill running.

use std::collections::HashSet;

use indexmap::IndexSet;
use serde::Deserialize;

use crate::format::versions::version_at_least;
use crate::js::JsValue;

/// What can start a monitor.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MonitorEvent {
    /// A session starting.
    SessionStart,
    /// Plugins reloading.
    PluginReload,
    /// A skill that ran.
    SkillInvoked {
        /// The skill that ran, as a trigger writes it after `on-skill-invoke:`: one of the names
        /// `skillTriggerNames` gives a skill of the monitor's plugin (judgment J-L11).
        skill: String,
    },
}

/// A skill that ran, as the session knows it (`SessionSkill`): its plugin and every name that invokes it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokedSkill {
    /// The id of the plugin it belongs to; absent for a folder or host skill.
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// Every name that invokes it, the namespaced `<plugin>:<skill>` first (`SessionSkill.names`).
    pub names: Vec<String>,
}

/// A name without its plugin namespace: `ops:deploy` is `deploy`, `ops:db:migrate` is `db:migrate`.
fn without_namespace(name: &str) -> Vec<String> {
    name.split_once(':')
        .map_or_else(Vec::new, |(_, rest)| vec![rest.to_owned()])
}

/// The names a monitor of plugin `pluginId` may write after `on-skill-invoke:` for this skill: none
/// unless the skill is that plugin's own; else each name the session gives it, namespaced
/// (`ops:deploy`) and without the plugin's namespace (`deploy`, the form of the docs' example).
/// Judgment J-L11 (spec/judgments.md).
#[must_use]
pub fn skill_trigger_names(plugin_id: &str, skill: &InvokedSkill) -> Vec<String> {
    if skill.plugin_id.as_deref() != Some(plugin_id) {
        return Vec::new();
    }
    let names: IndexSet<String> = skill
        .names
        .iter()
        .flat_map(|name| std::iter::once(name.clone()).chain(without_namespace(name)))
        .collect();
    names.into_iter().collect()
}

/// Whether the host runs a person's session; plugin monitors start only in an interactive one.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorMode {
    /// A person's session.
    pub interactive: bool,
    /// False on third-party providers or when telemetry/nonessential traffic disables flag fetching.
    #[serde(default)]
    pub feature_flags_available: Option<bool>,
    /// The host release, when it needs decisions for a release other than the tracked one.
    #[serde(default)]
    pub version: Option<String>,
}

/// components.md § Monitors: flag-free monitors require v2.1.310. Judgment: a host release that is
/// no version cannot establish the minimum.
fn version_allows_monitors(mode: &MonitorMode) -> bool {
    mode.feature_flags_available != Some(false)
        || version_at_least(mode.version.as_deref().unwrap_or(""), "2.1.310")
}

/// What can stop the monitors a session started.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum MonitorStopEvent {
    /// A plugin was disabled.
    #[serde(rename = "plugin-disabled")]
    PluginDisabled,
    /// The session ended.
    #[serde(rename = "session-ended")]
    SessionEnded,
}

/// Already running monitors survive disabling a plugin; only session end stops them.
#[must_use]
pub fn monitors_to_stop(started: Vec<JsValue>, event: MonitorStopEvent) -> Vec<JsValue> {
    match event {
        MonitorStopEvent::PluginDisabled => Vec::new(),
        MonitorStopEvent::SessionEnded => started,
    }
}

/// `"always"`: "the monitor starts at session start and on plugin reload".
const fn always_starts_on(event: &MonitorEvent) -> bool {
    match event {
        MonitorEvent::SessionStart | MonitorEvent::PluginReload => true,
        MonitorEvent::SkillInvoked { .. } => false,
    }
}

/// Per trigger: whether an event is the one it waits for.
fn starts(trigger: &JsValue, event: &MonitorEvent) -> bool {
    match trigger.get("kind").and_then(JsValue::as_str) {
        Some("always") => always_starts_on(event),
        // "With `on-skill-invoke:<skill>`, it starts the first time that skill runs."
        Some("on-skill-invoke") => match (trigger.get("skill"), event) {
            (Some(wanted), MonitorEvent::SkillInvoked { skill }) => {
                wanted.as_str() == Some(skill.as_str())
            }
            _ => false,
        },
        _ => false,
    }
}

/// The monitors an event starts (plugins/manifest-reference.md, "monitors"; plugins/components.md,
/// "Monitors"). Pure: the host keeps the set of names it has started and spawns what comes back.
///
/// - `always` starts on session start and on plugin reload; `on-skill-invoke:<skill>` only when that
///   skill runs, and "the first time": a name already in `started` never starts again.
/// - "Plugin monitors don't start in non-interactive mode with the `-p` flag": nothing starts there.
///   Judgment J-L9: the docs name only that flag, so `mode.interactive` is the kit's own input, which
///   the host sets false for `-p` and for any run with no person at it.
/// - Judgment J-L10: `started` holds the names of the monitors the host has running (or has started in
///   this session) for this plugin; none of them is returned again, for `always` as well, so a plugin
///   reload does not start a second copy ("if you disable a plugin mid-session, Claude Code doesn't
///   stop monitors that are already running"). A host that stopped one removes its name.
#[must_use]
pub fn monitors_to_start(
    monitors: Vec<JsValue>,
    event: &MonitorEvent,
    started: &HashSet<String>,
    mode: &MonitorMode,
) -> Vec<JsValue> {
    if !mode.interactive || !version_allows_monitors(mode) {
        return Vec::new();
    }
    monitors
        .into_iter()
        .filter(|monitor| {
            let named = monitor.get("name").and_then(JsValue::as_str);
            !named.is_some_and(|name| started.contains(name))
                && monitor.get("when").is_some_and(|when| starts(when, event))
        })
        .collect()
}

crate::registry::exports! {
    "loading/resolve/monitor-trigger" {
        "skillTriggerNames" => ok |plugin_id: String, skill: InvokedSkill|
            skill_trigger_names(&plugin_id, &skill);
        "monitorsToStop" => ok |started: crate::registry::Set<JsValue>, event: MonitorStopEvent|
            monitors_to_stop(started.members, event);
        "monitorsToStart" => ok |monitors: Vec<JsValue>, event: MonitorEvent, started: crate::registry::Set<String>, mode: MonitorMode|
            monitors_to_start(monitors, &event, &started.members.into_iter().collect(), &mode);
    }
}
