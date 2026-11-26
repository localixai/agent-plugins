//! Port of `src/cli/background-session.ts`, switched.
//!
//! What moves into a background session and what an agent view applies (agent-view.md, "What
//! carries over when you background", "Settings, plugins, and MCP servers"). Pure over the flags of
//! the launch and the work the host says is running: starting the background session, stopping the
//! work and showing the `Background this session?` dialog are the host's. `--fallback-model` and
//! `--allow-dangerously-skip-permissions` also carry over; they are not flags the kit plans, so the
//! host carries them itself.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::flags::types::{FlagName, GivenFlags};
use crate::js::in_key_order;

/// "Configuration flags from the original launch carry through to the backgrounded session".
const CARRIED: [FlagName; 6] = [
    FlagName::McpConfig,
    FlagName::StrictMcpConfig,
    FlagName::Settings,
    FlagName::SettingSources,
    FlagName::AddDir,
    FlagName::PluginDir,
];

/// "Agent view applies `--settings`, `--setting-sources`, and `--plugin-dir` to itself".
const AGENT_VIEW_OWN: [FlagName; 3] = [
    FlagName::Settings,
    FlagName::SettingSources,
    FlagName::PluginDir,
];

/// The flags of agent view's table, every one passed through to the sessions it dispatches.
const AGENT_VIEW_PASSED: [FlagName; 6] = CARRIED;

/// The text of a flag name, as it is a key of `GivenFlags`.
fn key_of(name: FlagName) -> &'static str {
    match name {
        FlagName::PluginDir => "--plugin-dir",
        FlagName::PluginUrl => "--plugin-url",
        FlagName::Bare => "--bare",
        FlagName::SafeMode => "--safe-mode",
        FlagName::Restricted => "--restricted",
        FlagName::SettingSources => "--setting-sources",
        FlagName::Settings => "--settings",
        FlagName::DisableSlashCommands => "--disable-slash-commands",
        FlagName::AddDir => "--add-dir",
        FlagName::Agent => "--agent",
        FlagName::Agents => "--agents",
        FlagName::AppendSubagentSystemPrompt => "--append-subagent-system-prompt",
        FlagName::AppendSubagentSystemPromptFile => "--append-subagent-system-prompt-file",
        FlagName::ForwardSubagentText => "--forward-subagent-text",
        FlagName::Channels => "--channels",
        FlagName::DangerouslyLoadDevelopmentChannels => "--dangerously-load-development-channels",
        FlagName::McpConfig => "--mcp-config",
        FlagName::StrictMcpConfig => "--strict-mcp-config",
        FlagName::PermissionMode => "--permission-mode",
        FlagName::Tools => "--tools",
    }
}

fn pick(given: &GivenFlags, names: &[FlagName]) -> GivenFlags {
    let mut kept = GivenFlags::new();
    for name in names {
        let key = key_of(*name);
        if let Some(value) = given.get(key).filter(|value| !value.is_undefined()) {
            kept.insert(key.to_owned(), value.clone());
        }
    }
    kept
}

/// The flags of the launch the background session starts with, so a plugin loaded by flag stays loaded.
#[must_use]
pub fn background_session_flags(given: &GivenFlags) -> GivenFlags {
    pick(given, &CARRIED)
}

/// What agent view loads with and what it passes on.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgentViewFlags {
    /// What agent view itself loads with.
    pub own: GivenFlags,
    /// What every session dispatched from it gets.
    pub dispatched: GivenFlags,
}

/// What agent view applies to itself and what it passes to the sessions it dispatches.
#[must_use]
pub fn agent_view_flags(given: &GivenFlags) -> AgentViewFlags {
    AgentViewFlags {
        own: pick(given, &AGENT_VIEW_OWN),
        dispatched: pick(given, &AGENT_VIEW_PASSED),
    }
}

/// The kinds of in-flight work the page names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackgroundWorkKind {
    /// A shell command.
    Shell,
    /// A subagent.
    Subagent,
    /// A loop.
    Loop,
    /// Artifact replies.
    ArtifactReplies,
    /// A workflow.
    Workflow,
    /// A monitor.
    Monitor,
}

/// "Carries over", "continues only when Claude relaunches it", or "stops".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundWorkFate {
    /// Carries over.
    Carries,
    /// Continues only when Claude relaunches it.
    Relaunch,
    /// Stops.
    Stops,
}

/// What becomes of each kind of work.
const fn fate_of(kind: BackgroundWorkKind) -> BackgroundWorkFate {
    match kind {
        BackgroundWorkKind::Shell
        | BackgroundWorkKind::Subagent
        | BackgroundWorkKind::Loop
        | BackgroundWorkKind::ArtifactReplies => BackgroundWorkFate::Carries,
        BackgroundWorkKind::Workflow => BackgroundWorkFate::Relaunch,
        BackgroundWorkKind::Monitor => BackgroundWorkFate::Stops,
    }
}

/// Whether a fate stops the work.
const fn stops(fate: BackgroundWorkFate) -> bool {
    matches!(fate, BackgroundWorkFate::Stops)
}

/// One piece of work that is running.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningWork {
    /// Its id.
    pub id: String,
    /// What kind of work it is.
    pub kind: BackgroundWorkKind,
    /// The subagent that started it, by id.
    #[serde(default)]
    pub started_by: Option<String>,
}

/// What backgrounding does to the running work.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BackgroundingPlan {
    /// What becomes of each piece of work, by id.
    pub fates: IndexMap<String, BackgroundWorkFate>,
    /// Some work stops: the user confirms the `Background this session?` dialog first.
    pub confirm: bool,
}

/// "A subagent moves together with everything it started, so it carries over only when all of that
/// work can move too": a subagent stops when anything it started, at any depth, stops.
#[must_use]
pub fn plan_backgrounding(work: &[RunningWork]) -> BackgroundingPlan {
    let mut fates: IndexMap<String, BackgroundWorkFate> = IndexMap::new();
    for item in work {
        fates.insert(item.id.clone(), fate_of(item.kind));
    }
    let mut changed = true;
    while changed {
        changed = false;
        for item in work {
            let Some(owner) = item.started_by.as_ref() else {
                continue;
            };
            let Some(owner_fate) = fates.get(owner).copied() else {
                continue;
            };
            let own = fates
                .get(&item.id)
                .copied()
                .unwrap_or(BackgroundWorkFate::Carries);
            if stops(own) && !stops(owner_fate) {
                fates.insert(owner.clone(), BackgroundWorkFate::Stops);
                changed = true;
            }
        }
    }
    let confirm = fates.values().any(|fate| stops(*fate));
    BackgroundingPlan {
        fates: in_key_order(fates),
        confirm,
    }
}

crate::registry::exports! {
    "cli/background-session" {
        "backgroundSessionFlags" => ok |given: GivenFlags| background_session_flags(&given);
        "agentViewFlags" => ok |given: GivenFlags| agent_view_flags(&given);
        "planBackgrounding" => ok |work: Vec<RunningWork>| plan_backgrounding(&work);
    }
}
