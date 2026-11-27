//! Port of `src/agents/hooks.ts`, switched.
//!
//! The hooks an agent's frontmatter defines, and which of them fire (sub-agents.md, "Hooks in
//! subagent frontmatter"). The kit selects; running them is the hooks engine's. It returns only the
//! agent's own groups: "in the main-session case they run alongside any hooks defined in
//! `settings.json`", so the host merges them with the settings' groups.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::agent_record::AgentSource;
use crate::js::JsValue;
use crate::rules::hook_matcher::{MatcherMode, matches_matcher};

/// How the agent runs: "spawned as a subagent" or "as the main session via `--agent`".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum AgentRunKind {
    /// `subagent`.
    #[serde(rename = "subagent")]
    Subagent,
    /// `main-session`.
    #[serde(rename = "main-session")]
    MainSession,
}

/// The groups of each event, each group as it was written (`HooksSettings`).
pub type HooksSettings = IndexMap<String, Vec<JsValue>>;

/// `Pick<AgentRecord, 'hooks' | 'source'>`.
#[derive(Clone, Debug, Deserialize)]
pub struct HookSource {
    /// The agent's frontmatter hooks.
    #[serde(default)]
    pub hooks: Option<HooksSettings>,
    /// Where the agent file comes from.
    pub source: AgentSource,
}

/// "When the agent is invoked as a subagent, `Stop` hooks in frontmatter are automatically
/// converted to `SubagentStop` events." The converted groups follow the file's own `SubagentStop`
/// groups (judgment: the docs give no order). `Stop` has no matcher input, so a `matcher` written
/// there is ignored (hooks.md, "Matcher patterns"); it is dropped in the conversion, or it would
/// start to filter on the agent type once the event is `SubagentStop` (judgment).
fn stop_to_subagent_stop(hooks: &HooksSettings) -> HooksSettings {
    let mut rest = hooks.clone();
    let Some(stop) = rest.shift_remove("Stop") else {
        return hooks.clone();
    };
    let own = rest.shift_remove("SubagentStop").unwrap_or_default();
    let converted = stop.iter().map(|group| {
        JsValue::from([(
            "hooks",
            group.get("hooks").cloned().unwrap_or(JsValue::Undefined),
        )])
    });
    rest.insert(
        "SubagentStop".to_owned(),
        own.into_iter().chain(converted).collect(),
    );
    rest
}

fn keep_as_written(hooks: &HooksSettings) -> HooksSettings {
    hooks.clone()
}

fn run_hooks(run: AgentRunKind, hooks: &HooksSettings) -> HooksSettings {
    match run {
        AgentRunKind::Subagent => stop_to_subagent_stop(hooks),
        AgentRunKind::MainSession => keep_as_written(hooks),
    }
}

/// Whether a source's frontmatter hooks apply at all: "Ignored for plugin subagents".
const fn hooks_apply(source: AgentSource) -> bool {
    match source {
        AgentSource::Managed | AgentSource::Flag | AgentSource::Project | AgentSource::User => true,
        AgentSource::Plugin => false,
    }
}

/// What the agent's hooks are for this run. A plugin agent has none: Claude Code ignores `hooks`
/// "for security reasons" (the reader already drops them, and this holds for a record built another
/// way). The record is not changed.
#[must_use]
pub fn agent_hook_groups(agent: &HookSource, run: AgentRunKind) -> HooksSettings {
    match &agent.hooks {
        Some(hooks) if hooks_apply(agent.source) => run_hooks(run, hooks),
        Some(_) | None => HooksSettings::new(),
    }
}

/// Which sources need the folder the agent file came from to be trusted: only a project-level agent
/// (which here also stands for an `--add-dir` one). "Hooks from user-level subagents in
/// `~/.claude/agents/` and from definitions you pass with `--agents` run without this step"; managed
/// agents are exempt too (judgment: organization-installed, never from a folder being opened).
const fn needs_trust(source: AgentSource) -> bool {
    match source {
        AgentSource::Project => true,
        AgentSource::Managed | AgentSource::Flag | AgentSource::User | AgentSource::Plugin => false,
    }
}

/// The verdict of `hooks_trust_gate`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TrustGate {
    /// `run`.
    #[serde(rename = "run")]
    Run,
    /// `skip-untrusted`.
    #[serde(rename = "skip-untrusted")]
    SkipUntrusted,
}

/// "Until you trust the folder, the subagent still runs, but Claude Code skips its frontmatter
/// hooks." `folderTrusted` is the host's one answer to "did the user accept the trust dialog for
/// this very folder": a parent folder's trust and a `-p` session do not make it true, which is why
/// the kit does not compute it.
#[must_use]
pub const fn hooks_trust_gate(source: AgentSource, folder_trusted: bool) -> TrustGate {
    if needs_trust(source) && !folder_trusted {
        TrustGate::SkipUntrusted
    } else {
        TrustGate::Run
    }
}

/// How an event's matcher is read (hooks.md, "Matcher patterns"): `none` for the events that "have no
/// matcher support" and fire on every occurrence; `narrow` for `FileChanged` and `StopFailure`; every
/// other event matches its own input with the standard grammar.
const MATCHING: [(&str, Option<MatcherMode>); 33] = [
    ("PreToolUse", Some(MatcherMode::Standard)),
    ("PostToolUse", Some(MatcherMode::Standard)),
    ("PostToolUseFailure", Some(MatcherMode::Standard)),
    ("PostToolBatch", None),
    ("Notification", Some(MatcherMode::Standard)),
    ("UserPromptSubmit", None),
    ("UserPromptExpansion", Some(MatcherMode::Standard)),
    ("SessionStart", Some(MatcherMode::Standard)),
    ("SessionEnd", Some(MatcherMode::Standard)),
    ("Stop", None),
    ("StopFailure", Some(MatcherMode::Narrow)),
    ("SubagentStart", Some(MatcherMode::Standard)),
    ("SubagentStop", Some(MatcherMode::Standard)),
    ("PreCompact", Some(MatcherMode::Standard)),
    ("PostCompact", Some(MatcherMode::Standard)),
    ("PreModelSwitch", Some(MatcherMode::Standard)),
    ("PostModelSwitch", Some(MatcherMode::Standard)),
    ("PermissionRequest", Some(MatcherMode::Standard)),
    ("PermissionDenied", Some(MatcherMode::Standard)),
    ("Setup", Some(MatcherMode::Standard)),
    ("TeammateIdle", None),
    ("TaskCreated", None),
    ("TaskCompleted", None),
    ("Elicitation", Some(MatcherMode::Standard)),
    ("ElicitationResult", Some(MatcherMode::Standard)),
    ("ConfigChange", Some(MatcherMode::Standard)),
    ("WorktreeCreate", None),
    ("WorktreeRemove", None),
    ("InstructionsLoaded", Some(MatcherMode::Standard)),
    ("CwdChanged", None),
    ("FileChanged", Some(MatcherMode::Narrow)),
    ("DirectoryAdded", Some(MatcherMode::Standard)),
    ("MessageDisplay", None),
];

/// "A `PreToolUse` or `PermissionRequest` hook with a `Bash` matcher also runs on shell commands
/// Claude starts with the Monitor tool", while a `PostToolUse` hook with one "doesn't run for
/// Monitor" (hooks.md, "Match Monitor commands"). So a Monitor call is matched under its own name
/// and, on these two events only, as `Bash`: `Bash|PowerShell`, `^Bash$` and `Bash.*` select it, and
/// so do `Bash|Monitor`, `.*` and no matcher.
const MONITOR_ACTS_AS_BASH: [&str; 2] = ["PreToolUse", "PermissionRequest"];

const MONITOR_TOOL: &str = "Monitor";

fn matcher_inputs<'a>(event: &str, value: Option<&'a str>) -> Vec<Option<&'a str>> {
    if value == Some(MONITOR_TOOL) && MONITOR_ACTS_AS_BASH.contains(&event) {
        vec![value, Some("Bash")]
    } else {
        vec![value]
    }
}

fn group_matches(group: &JsValue, event: &str, value: Option<&str>) -> bool {
    let mode = MATCHING
        .iter()
        .find(|(name, _)| *name == event)
        .map(|(_, mode)| *mode);
    // An event the table does not know reads with the standard grammar, as the matcher's default.
    let Some(mode) = mode.unwrap_or(Some(MatcherMode::Standard)) else {
        return true;
    };
    let matcher = group.get("matcher").and_then(JsValue::as_str);
    matcher_inputs(event, value)
        .into_iter()
        .any(|input| matches_matcher(matcher, input, mode))
}

/// Why an agent that has hooks selected none: not active, or its folder is not trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Skipped {
    /// The folder is not trusted.
    Untrusted,
    /// The agent is not active.
    Inactive,
}

/// The groups to run and why none.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgentHookSelection {
    /// The groups to run, in the order the file lists them.
    pub groups: Vec<JsValue>,
    /// Why an agent that has hooks selected none: not active, or its folder is not trusted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<Skipped>,
}

/// What one occurrence of an event is.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHookContext {
    /// How the agent runs.
    pub run: AgentRunKind,
    /// The event that occurred; for a subagent's `Stop`, `SubagentStop`.
    pub event: String,
    /// The event's matcher input (the tool name, the agent type, ...); none for `Stop`. A Monitor
    /// call is passed as `Monitor`; the kit adds the `Bash` reading where the docs give it.
    #[serde(default)]
    pub matcher_value: Option<String>,
    /// "These hooks only run while that specific subagent is active."
    pub active: bool,
    /// See `hooks_trust_gate`.
    pub folder_trusted: bool,
}

/// The groups of the agent's frontmatter that fire for one occurrence of an event. Judgments: an
/// inactive agent is reported before an untrusted one, and a skip is reported only for an agent
/// that has hooks, so the host logs the trust error only where hooks were lost.
#[must_use]
pub fn select_agent_hooks(agent: &HookSource, ctx: &AgentHookContext) -> AgentHookSelection {
    let all = agent_hook_groups(agent, ctx.run);
    if all.is_empty() {
        return AgentHookSelection {
            groups: Vec::new(),
            skipped: None,
        };
    }
    if !ctx.active {
        return AgentHookSelection {
            groups: Vec::new(),
            skipped: Some(Skipped::Inactive),
        };
    }
    if hooks_trust_gate(agent.source, ctx.folder_trusted) == TrustGate::SkipUntrusted {
        return AgentHookSelection {
            groups: Vec::new(),
            skipped: Some(Skipped::Untrusted),
        };
    }
    let groups = all
        .get(&ctx.event)
        .map(|groups| {
            groups
                .iter()
                .filter(|group| group_matches(group, &ctx.event, ctx.matcher_value.as_deref()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    AgentHookSelection {
        groups,
        skipped: None,
    }
}

crate::registry::exports! {
    "agents/hooks" {
        "agentHookGroups" => ok |agent: HookSource, run: AgentRunKind| agent_hook_groups(&agent, run);
        "hooksTrustGate" => ok |source: AgentSource, folder_trusted: bool| hooks_trust_gate(source, folder_trusted);
        "selectAgentHooks" => ok |agent: HookSource, ctx: AgentHookContext| select_agent_hooks(&agent, &ctx);
    }
}
