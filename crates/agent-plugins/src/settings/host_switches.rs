//! Port of `src/settings/host-switches.ts`, switched.
//!
//! The host-wide environment variables that switch a feature on or off or bound it (env-vars.md,
//! "Variables"), read through the `Environment` port (L3). Each function is the decision the table
//! row describes; the host applies it.

use serde::{Deserialize, Serialize};

use crate::format::env_flag::is_truthy_env;
use crate::format::env_number::positive_whole_env;
use crate::js::{is_safe_integer, number, trim};
use crate::ports::environment::Environment;
use crate::registry::Port;

async fn text(env: &dyn Environment, name: &str) -> String {
    trim(&env.get(name).await.unwrap_or_default()).to_owned()
}

/// An on/off variable: `1`, `true`, `yes` or `on` in any casing (env-vars.md; spec §3.6.1,
/// J3.16).
async fn is_on(env: &dyn Environment, name: &str) -> bool {
    is_truthy_env(env.get(name).await.as_deref())
}

/// `CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL=1`: "skips the registration permanently".
pub async fn official_marketplace_autoinstall_enabled(env: &dyn Environment) -> bool {
    !is_on(env, "CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL").await
}

/// What `permission_prompt_notify_hooks_enabled` reads of the session.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSession {
    /// Whether permission requests go to the Agent SDK's `canUseTool` callback.
    pub uses_can_use_tool: bool,
}

/// What the background refresh and the synchronous install read of the session.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeSession {
    /// Whether the session is interactive.
    pub interactive: bool,
}

/// `CLAUDE_CODE_DISABLE_PERMISSION_PROMPT_NOTIFY_HOOKS=1` stops `Notification` hooks for
/// unanswered permission requests "in sessions where Claude Code sends them to the Agent SDK's
/// `canUseTool` callback"; "has no effect in terminal sessions".
pub async fn permission_prompt_notify_hooks_enabled(
    env: &dyn Environment,
    session: &PromptSession,
) -> bool {
    !(session.uses_can_use_tool
        && is_on(env, "CLAUDE_CODE_DISABLE_PERMISSION_PROMPT_NOTIFY_HOOKS").await)
}

/// `CLAUDE_CODE_DISABLE_POLICY_SKILLS=1`: skip "the system-wide managed skills directory".
pub async fn policy_skills_enabled(env: &dyn Environment) -> bool {
    !is_on(env, "CLAUDE_CODE_DISABLE_POLICY_SKILLS").await
}

/// `CLAUDE_CODE_ENABLE_BACKGROUND_PLUGIN_REFRESH=1` refreshes plugin state at turn boundaries "in
/// non-interactive mode"; off by default.
pub async fn background_plugin_refresh_enabled(
    env: &dyn Environment,
    session: ModeSession,
) -> bool {
    !session.interactive && is_on(env, "CLAUDE_CODE_ENABLE_BACKGROUND_PLUGIN_REFRESH").await
}

/// "Accepts a positive whole number in plain digits; anything else is ignored" (default 20).
pub const DEFAULT_MAX_CONCURRENT_SUBAGENTS: f64 = 20.0;

/// The most subagents that run at once.
pub async fn max_concurrent_subagents(env: &dyn Environment) -> f64 {
    let value = text(env, "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS").await;
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
pub async fn plugin_git_timeout_ms(env: &dyn Environment) -> f64 {
    positive_whole_env(
        env.get("CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS")
            .await
            .as_deref(),
    )
    .unwrap_or(DEFAULT_PLUGIN_GIT_TIMEOUT_MS)
}

/// `CLAUDE_CODE_PLUGIN_PREFER_HTTPS=1`: clone GitHub `owner/repo` shorthand over HTTPS, not SSH.
pub async fn prefer_https(env: &dyn Environment) -> bool {
    is_on(env, "CLAUDE_CODE_PLUGIN_PREFER_HTTPS").await
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
pub async fn sync_plugin_install(env: &dyn Environment, session: ModeSession) -> SyncInstallWait {
    if session.interactive || !is_on(env, "CLAUDE_CODE_SYNC_PLUGIN_INSTALL").await {
        return SyncInstallWait {
            wait: false,
            timeout_ms: None,
        };
    }
    SyncInstallWait {
        wait: true,
        timeout_ms: positive_whole_env(
            env.get("CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS")
                .await
                .as_deref(),
        ),
    }
}

/// Whether telemetry is off. `DISABLE_TELEMETRY`: "any non-empty value ... Setting it to `0` or
/// `false` still opts out". `DO_NOT_TRACK` is "a standard boolean, so `0` leaves telemetry on".
pub async fn telemetry_opted_out(env: &dyn Environment) -> bool {
    if !env
        .get("DISABLE_TELEMETRY")
        .await
        .unwrap_or_default()
        .is_empty()
    {
        return true;
    }
    is_on(env, "DO_NOT_TRACK").await
}

/// A platform's path style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostPlatform {
    /// Unix paths.
    Posix,
    /// Windows paths.
    Windows,
}

/// `CLAUDE_CODE_PLUGIN_SEED_DIR`: "one or more read-only plugin seed directories, separated by `:`
/// on Unix or `;` on Windows"; "Claude Code uses the first seed that contains a given marketplace
/// or plugin cache", so the order is kept. Empty entries are dropped (judgment).
pub async fn plugin_seed_directories(env: &dyn Environment, platform: HostPlatform) -> Vec<String> {
    let separator = match platform {
        HostPlatform::Windows => ';',
        HostPlatform::Posix => ':',
    };
    text(env, "CLAUDE_CODE_PLUGIN_SEED_DIR")
        .await
        .split(separator)
        .map(|entry| trim(entry).to_owned())
        .filter(|entry| !entry.is_empty())
        .collect()
}

/// What the host knows of its temporary directory.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub async fn claude_temp_directory(env: &dyn Environment, input: &TempDirectoryInput) -> String {
    let set = text(env, "CLAUDE_CODE_TMPDIR").await;
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

crate::registry::exports! {
    "settings/host-switches" {
        "officialMarketplaceAutoinstallEnabled" => async ok |env: Port|
            official_marketplace_autoinstall_enabled(&env).await;
        "permissionPromptNotifyHooksEnabled" => async ok |env: Port, session: PromptSession|
            permission_prompt_notify_hooks_enabled(&env, &session).await;
        "policySkillsEnabled" => async ok |env: Port| policy_skills_enabled(&env).await;
        "backgroundPluginRefreshEnabled" => async ok |env: Port, session: ModeSession|
            background_plugin_refresh_enabled(&env, session).await;
        "DEFAULT_MAX_CONCURRENT_SUBAGENTS" => ok || DEFAULT_MAX_CONCURRENT_SUBAGENTS;
        "maxConcurrentSubagents" => async ok |env: Port| max_concurrent_subagents(&env).await;
        "DEFAULT_PLUGIN_GIT_TIMEOUT_MS" => ok || DEFAULT_PLUGIN_GIT_TIMEOUT_MS;
        "pluginGitTimeoutMs" => async ok |env: Port| plugin_git_timeout_ms(&env).await;
        "preferHttps" => async ok |env: Port| prefer_https(&env).await;
        "syncPluginInstall" => async ok |env: Port, session: ModeSession|
            sync_plugin_install(&env, session).await;
        "telemetryOptedOut" => async ok |env: Port| telemetry_opted_out(&env).await;
        "pluginSeedDirectories" => async ok |env: Port, platform: HostPlatform|
            plugin_seed_directories(&env, platform).await;
        "claudeTempDirectory" => async ok |env: Port, input: TempDirectoryInput|
            claude_temp_directory(&env, &input).await;
    }
}
