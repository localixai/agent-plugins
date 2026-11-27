//! Port of `src/settings/host-switches.ts` at sha256:8df94bada3ee.
//!
//! The host-wide environment variables that switch a feature on or off or bound it (env-vars.md,
//! "Variables"), read through the `Environment` port (L3). Each function is the decision the table
//! row describes; the host applies it. Takes the host's `Environment`, a synchronous object: stays
//! TypeScript in the package (the module comment of `crate::settings`), so it exports nothing.

use serde::Serialize;

use crate::format::env_flag::is_truthy_env;
use crate::format::env_number::positive_whole_env;
use crate::js::{is_safe_integer, number, trim};
use crate::ports::environment::Environment;

fn text(env: &dyn Environment, name: &str) -> String {
    trim(&env.get(name).unwrap_or_default()).to_owned()
}

/// An on/off variable: `1`, `true`, `yes` or `on` in any casing (env-vars.md; spec §3.6.1,
/// J3.16).
fn is_on(env: &dyn Environment, name: &str) -> bool {
    is_truthy_env(env.get(name).as_deref())
}

/// `CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL=1`: "skips the registration permanently".
#[must_use]
pub fn official_marketplace_autoinstall_enabled(env: &dyn Environment) -> bool {
    !is_on(env, "CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL")
}

/// `CLAUDE_CODE_DISABLE_PERMISSION_PROMPT_NOTIFY_HOOKS=1` stops `Notification` hooks for
/// unanswered permission requests "in sessions where Claude Code sends them to the Agent SDK's
/// `canUseTool` callback"; "has no effect in terminal sessions".
#[must_use]
pub fn permission_prompt_notify_hooks_enabled(
    env: &dyn Environment,
    uses_can_use_tool: bool,
) -> bool {
    !(uses_can_use_tool && is_on(env, "CLAUDE_CODE_DISABLE_PERMISSION_PROMPT_NOTIFY_HOOKS"))
}

/// `CLAUDE_CODE_DISABLE_POLICY_SKILLS=1`: skip "the system-wide managed skills directory".
#[must_use]
pub fn policy_skills_enabled(env: &dyn Environment) -> bool {
    !is_on(env, "CLAUDE_CODE_DISABLE_POLICY_SKILLS")
}

/// `CLAUDE_CODE_ENABLE_BACKGROUND_PLUGIN_REFRESH=1` refreshes plugin state at turn boundaries "in
/// non-interactive mode"; off by default.
#[must_use]
pub fn background_plugin_refresh_enabled(env: &dyn Environment, interactive: bool) -> bool {
    !interactive && is_on(env, "CLAUDE_CODE_ENABLE_BACKGROUND_PLUGIN_REFRESH")
}

/// "Accepts a positive whole number in plain digits; anything else is ignored" (default 20).
pub const DEFAULT_MAX_CONCURRENT_SUBAGENTS: f64 = 20.0;

/// The most subagents that run at once.
#[must_use]
pub fn max_concurrent_subagents(env: &dyn Environment) -> f64 {
    let value = text(env, "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS");
    let parsed = if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        number(&value)
    } else {
        0.0
    };
    if parsed > 0.0 && is_safe_integer(parsed) {
        parsed
    } else {
        DEFAULT_MAX_CONCURRENT_SUBAGENTS
    }
}

/// `CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS`: "Timeout in milliseconds for cloning or refreshing a
/// plugin marketplace (default: 120000)". A value that is not a positive whole number (digits,
/// `2e3`, `64_000`: spec §3.6.1 item 2, J3.17) leaves the default.
pub const DEFAULT_PLUGIN_GIT_TIMEOUT_MS: f64 = 120_000.0;

/// The timeout of a marketplace clone or refresh.
#[must_use]
pub fn plugin_git_timeout_ms(env: &dyn Environment) -> f64 {
    positive_whole_env(env.get("CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS").as_deref())
        .unwrap_or(DEFAULT_PLUGIN_GIT_TIMEOUT_MS)
}

/// `CLAUDE_CODE_PLUGIN_PREFER_HTTPS=1`: clone GitHub `owner/repo` shorthand over HTTPS, not SSH.
#[must_use]
pub fn prefer_https(env: &dyn Environment) -> bool {
    is_on(env, "CLAUDE_CODE_PLUGIN_PREFER_HTTPS")
}

/// The wait before the first query in a non-interactive run.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncInstallWait {
    /// Whether to wait.
    pub wait: bool,
    /// No default: without the timeout variable, "synchronous installation waits until complete".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<f64>,
}

/// `CLAUDE_CODE_SYNC_PLUGIN_INSTALL=1` in non-interactive mode waits for installation;
/// `CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS` bounds the wait. Judgment: an invalid timeout is
/// no bound.
#[must_use]
pub fn sync_plugin_install(env: &dyn Environment, interactive: bool) -> SyncInstallWait {
    if interactive || !is_on(env, "CLAUDE_CODE_SYNC_PLUGIN_INSTALL") {
        return SyncInstallWait {
            wait: false,
            timeout_ms: None,
        };
    }
    SyncInstallWait {
        wait: true,
        timeout_ms: positive_whole_env(
            env.get("CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS")
                .as_deref(),
        ),
    }
}

/// Whether telemetry is off. `DISABLE_TELEMETRY`: "any non-empty value ... Setting it to `0` or
/// `false` still opts out". `DO_NOT_TRACK` is "a standard boolean, so `0` leaves telemetry on".
#[must_use]
pub fn telemetry_opted_out(env: &dyn Environment) -> bool {
    if !env.get("DISABLE_TELEMETRY").unwrap_or_default().is_empty() {
        return true;
    }
    is_on(env, "DO_NOT_TRACK")
}

/// A platform's path style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostPlatform {
    /// Unix paths.
    Posix,
    /// Windows paths.
    Windows,
}

/// `CLAUDE_CODE_PLUGIN_SEED_DIR`: "one or more read-only plugin seed directories, separated by `:`
/// on Unix or `;` on Windows"; "Claude Code uses the first seed that contains a given marketplace
/// or plugin cache", so the order is kept. Empty entries are dropped (judgment).
#[must_use]
pub fn plugin_seed_directories(env: &dyn Environment, platform: HostPlatform) -> Vec<String> {
    let separator = match platform {
        HostPlatform::Windows => ';',
        HostPlatform::Posix => ':',
    };
    text(env, "CLAUDE_CODE_PLUGIN_SEED_DIR")
        .split(separator)
        .map(|entry| trim(entry).to_owned())
        .filter(|entry| !entry.is_empty())
        .collect()
}

/// What the host knows of its temporary directory.
#[derive(Clone, Debug, PartialEq)]
pub struct TempDirectoryInput {
    /// The platform's path style.
    pub platform: HostPlatform,
    /// The user id.
    pub uid: f64,
    /// The host's default temporary directory.
    pub fallback: String,
}

/// `CLAUDE_CODE_TMPDIR`: "Claude Code appends `/claude-{uid}/` to this path on Unix, or `/claude/`
/// on Windows". Without the override the host's default stands ("`/tmp` on macOS, `os.tmpdir()` on
/// Linux and Windows"), which the host passes as `fallback`. The uid is the host's.
#[must_use]
pub fn claude_temp_directory(env: &dyn Environment, input: &TempDirectoryInput) -> String {
    let set = text(env, "CLAUDE_CODE_TMPDIR");
    let chosen = if set.is_empty() {
        &input.fallback
    } else {
        &set
    };
    let base = chosen.trim_end_matches(['/', '\\']);
    match input.platform {
        HostPlatform::Windows => format!("{base}/claude/"),
        HostPlatform::Posix => format!("{base}/claude-{}/", crate::js::number_to_string(input.uid)),
    }
}
