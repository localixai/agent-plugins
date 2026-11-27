//! Port of `src/agents/launch.ts`, switched.
//!
//! Small pure decisions a host takes when it starts a subagent, one per frontmatter field that
//! changes how the run is set up (sub-agents.md, "Frontmatter reference", "Run subagents in
//! foreground or background", "What loads at startup", "Set the auto-compact window for one
//! subagent"). The kit decides; compacting, running the turn loop, creating a worktree and loading
//! files are the host's. The model and effort decisions are in `launch-model.ts`.

use serde::{Deserialize, Serialize};

use crate::format::agent_scalars::AUTO_COMPACT_RANGE;
use crate::format::env_flag::is_falsy_env;
use crate::js::{is_safe_integer, math_min, trim};

fn window_min() -> f64 {
    f64::from(AUTO_COMPACT_RANGE.min)
}

fn window_max() -> f64 {
    f64::from(AUTO_COMPACT_RANGE.max)
}

/// What the auto-compact window is decided from.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCompactInput {
    /// The definition's `autoCompactWindow` (already read; a dropped field is undefined).
    #[serde(default)]
    pub field: Option<f64>,
    /// The window the subagent would otherwise inherit.
    pub inherited: f64,
    /// `CLAUDE_CODE_AUTO_COMPACT_WINDOW` as a number.
    #[serde(default)]
    pub env: Option<f64>,
    /// The agent runs the whole session (`--agent`, the `agent` setting).
    pub as_main_session: bool,
}

fn clamp(tokens: f64) -> f64 {
    math_min(tokens.max(window_min()), window_max())
}

/// model-config.md: a value below the minimum or above the maximum is treated as the limit.
fn usable_env(env: Option<f64>) -> Option<f64> {
    env.filter(|env| env.is_finite() && *env > 0.0).map(clamp)
}

fn in_range(tokens: f64) -> bool {
    is_safe_integer(tokens) && tokens >= window_min() && tokens <= window_max()
}

/// The auto-compact window a subagent compacts at. The variable wins ("the subagent uses that
/// variable's window instead and the field has no effect, unless Claude Code ignores the variable's
/// value"); the field only lowers ("To compact earlier, set a value below that window") and "has no
/// effect" for a session run as the agent.
#[must_use]
pub fn effective_auto_compact_window(input: &AutoCompactInput) -> f64 {
    if let Some(env) = usable_env(input.env) {
        return env;
    }
    let inherited = input.inherited;
    match input.field {
        Some(field) if !input.as_main_session && in_range(field) => math_min(field, inherited),
        Some(_) | None => inherited,
    }
}

/// Whether the subagent stopped, and whether its output is partial.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TurnLimitOutcome {
    /// Reached.
    pub stop: bool,
    /// "Claude Code returns its output marked as partial" and Claude can resume it.
    pub partial: bool,
}

/// Whether the subagent has reached `maxTurns`: "Maximum number of agentic turns before the
/// subagent stops". No limit unless it is a positive whole number (judgment, spec/judgments.md).
#[must_use]
pub fn turn_limit_outcome(max_turns: Option<f64>, turns_taken: f64) -> TurnLimitOutcome {
    let reached = max_turns.is_some_and(|max_turns| {
        max_turns.is_finite()
            && max_turns.fract().abs() < f64::EPSILON
            && max_turns >= 1.0
            && turns_taken >= max_turns
    });
    TurnLimitOutcome {
        stop: reached,
        partial: reached,
    }
}

const LONG_TTL: &str = "1h";

/// What the frontmatter's `cacheTtl` is decided from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheTtlInput {
    /// The definition's `cacheTtl`.
    #[serde(default)]
    pub cache_ttl: Option<String>,
    /// The Claude subscription is using usage credits.
    pub using_usage_credits: bool,
}

/// The `cacheTtl` the frontmatter contributes to a subagent's requests: it "ignores `1h` while your
/// Claude subscription is using usage credits" (prompt-caching.md, "Choose the TTL yourself"). The
/// result is the frontmatter's step of the precedence; `undefined` leaves the decision to the next one.
#[must_use]
pub fn effective_cache_ttl(input: &CacheTtlInput) -> Option<String> {
    if input.using_usage_credits && input.cache_ttl.as_deref() == Some(LONG_TTL) {
        None
    } else {
        input.cache_ttl.clone()
    }
}

/// What "runs in the background" is decided from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundInput {
    /// The definition's `background`.
    #[serde(default)]
    pub background: Option<bool>,
    /// Claude asked to wait for the result (on its judgment or yours in `claude -p` and the SDK; fork mode off).
    pub claude_will_wait: bool,
    /// `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1`: "Claude waits for subagents to finish".
    #[serde(default)]
    pub background_disabled: Option<bool>,
    /// The main conversation is an interactive terminal, VS Code or desktop session on the user's
    /// machine (not `-p` or the Agent SDK): with fork mode on it cannot wait for a subagent.
    #[serde(default)]
    pub interactive: Option<bool>,
    /// `CLAUDE_CODE_FORK_SUBAGENT` as set: fork mode is on unless it is an off spelling (`0`).
    #[serde(default)]
    pub fork_subagent: Option<String>,
}

/// Fork mode: on by default, `CLAUDE_CODE_FORK_SUBAGENT=0` (any off spelling) turns it off (spec §5.12.3).
#[must_use]
pub fn fork_mode_on(value: Option<&str>) -> bool {
    !is_falsy_env(value)
}

/// Whether a subagent runs in the background. "With nothing configured, a subagent that Claude
/// spawns from your main conversation starts in the background"; `background: true` keeps it there
/// "even when Claude asks to wait for it". With fork mode on, an interactive main conversation cannot
/// wait, so the subagent starts in the background there (env-vars.md, `CLAUDE_CODE_FORK_SUBAGENT`).
#[must_use]
pub fn runs_in_background(input: &BackgroundInput) -> bool {
    if input.background_disabled == Some(true) {
        return false;
    }
    let cannot_wait =
        input.interactive == Some(true) && fork_mode_on(input.fork_subagent.as_deref());
    input.background == Some(true) || cannot_wait || !input.claude_will_wait
}

/// The CLAUDE.md files a subagent starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's JSON shape: one flag per CLAUDE.md scope"
)]
pub struct ClaudeMdScopes {
    /// The user's.
    pub user: bool,
    /// The project's.
    pub project: bool,
    /// The local one.
    pub local: bool,
    /// The managed policy's.
    pub managed_policy: bool,
    /// A subdirectory's CLAUDE.md and matching path-scoped rules, loaded when the subagent works with files there.
    pub nested: bool,
}

const ALL_CLAUDE_MD: ClaudeMdScopes = ClaudeMdScopes {
    user: true,
    project: true,
    local: true,
    managed_policy: true,
    nested: true,
};

/// What the CLAUDE.md scopes are decided from.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeMdInput {
    /// The definition's `omitClaudeMd`.
    #[serde(default)]
    pub omit_claude_md: Option<bool>,
    /// The agent runs as the main session agent.
    pub as_main_session: bool,
    /// The subagent is a managed one.
    pub managed_subagent: bool,
}

/// The CLAUDE.md files a subagent starts with. `omitClaudeMd` drops the user, project and local
/// files; "managed policy files still load, except for managed subagents"; it is "Ignored when the
/// agent runs as the main session agent". A subdirectory's CLAUDE.md and matching path-scoped rules
/// "still load when the subagent works with files there", with or without `omitClaudeMd` (`nested`).
#[must_use]
pub fn claude_md_scopes(input: &ClaudeMdInput) -> ClaudeMdScopes {
    if input.omit_claude_md != Some(true) || input.as_main_session {
        return ALL_CLAUDE_MD;
    }
    ClaudeMdScopes {
        user: false,
        project: false,
        local: false,
        managed_policy: !input.managed_subagent,
        nested: true,
    }
}

/// What happens to a subagent's worktree when it finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorktreeDisposition {
    /// No worktree.
    None,
    /// Removed.
    Cleanup,
    /// Kept.
    Keep,
}

/// What the worktree's fate is decided from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInput {
    /// The definition's `isolation`.
    #[serde(default)]
    pub isolation: Option<String>,
    /// The subagent made changes.
    pub has_changes: bool,
}

/// What happens to a subagent's worktree when it finishes. Only `isolation: worktree` has one
/// (sub-agents.md, `isolation`); it is "automatically cleaned up if the subagent makes no changes",
/// and one with changes "stays on disk until the periodic sweep" (worktrees.md, "Isolate subagents
/// with worktrees"). Without `isolation` there is no worktree: `none`.
#[must_use]
pub fn worktree_disposition(input: &WorktreeInput) -> WorktreeDisposition {
    if input.isolation.as_deref() != Some("worktree") {
        return WorktreeDisposition::None;
    }
    if input.has_changes {
        WorktreeDisposition::Keep
    } else {
        WorktreeDisposition::Cleanup
    }
}

/// Judgment: the docs say "Prepended to any user-provided prompt" and no separator; a blank line.
const PROMPT_SEPARATOR: &str = "\n\n";

/// What the first turn is decided from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstTurnInput {
    /// The definition's `initialPrompt`.
    #[serde(default)]
    pub initial_prompt: Option<String>,
    /// The agent runs as the main session agent.
    pub as_main_session: bool,
    /// What the user typed.
    #[serde(default)]
    pub user_prompt: Option<String>,
}

/// The first user turn to submit for an agent run as the main session agent: its `initialPrompt`,
/// "Prepended to any user-provided prompt". A subagent run submits nothing (undefined). Processing
/// commands and skills in the text is the host's.
#[must_use]
pub fn main_session_first_turn(input: &FirstTurnInput) -> Option<String> {
    let initial = input.initial_prompt.as_deref().map(trim)?;
    if !input.as_main_session || initial.is_empty() {
        return None;
    }
    match input.user_prompt.as_deref().map(trim) {
        None | Some("") => Some(initial.to_owned()),
        Some(user) => Some(format!("{initial}{PROMPT_SEPARATOR}{user}")),
    }
}

crate::registry::exports! {
    "agents/launch" {
        "effectiveAutoCompactWindow" => ok |input: AutoCompactInput| effective_auto_compact_window(&input);
        "turnLimitOutcome" => ok |max_turns: Option<f64>, turns_taken: f64| turn_limit_outcome(max_turns, turns_taken);
        "effectiveCacheTtl" => ok |input: CacheTtlInput| effective_cache_ttl(&input);
        "forkModeOn" => ok |value: Option<String>| fork_mode_on(value.as_deref());
        "runsInBackground" => ok |input: BackgroundInput| runs_in_background(&input);
        "claudeMdScopes" => ok |input: ClaudeMdInput| claude_md_scopes(&input);
        "worktreeDisposition" => ok |input: WorktreeInput| worktree_disposition(&input);
        "mainSessionFirstTurn" => ok |input: FirstTurnInput| main_session_first_turn(&input);
    }
}
