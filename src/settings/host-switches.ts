/**
 * The host-wide environment variables that switch a feature on or off or bound it (env-vars.md,
 * "Variables"), read through the `Environment` port (L3). Each function is the decision the table row
 * describes; the host applies it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/host_switches.rs`, docs/RUST.md).
 */
import type { Environment } from '../ports/environment.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('settings/host-switches');

/** `CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL=1`: "skips the registration permanently". */
export const officialMarketplaceAutoinstallEnabled = (env: Environment): boolean =>
  call('officialMarketplaceAutoinstallEnabled', env);

/**
 * `CLAUDE_CODE_DISABLE_PERMISSION_PROMPT_NOTIFY_HOOKS=1` stops `Notification` hooks for unanswered
 * permission requests "in sessions where Claude Code sends them to the Agent SDK's `canUseTool`
 * callback"; "has no effect in terminal sessions".
 */
export const permissionPromptNotifyHooksEnabled = (
  env: Environment,
  session: { readonly usesCanUseTool: boolean },
): boolean =>
  call('permissionPromptNotifyHooksEnabled', env, { usesCanUseTool: session.usesCanUseTool });

/** `CLAUDE_CODE_DISABLE_POLICY_SKILLS=1`: skip "the system-wide managed skills directory". */
export const policySkillsEnabled = (env: Environment): boolean => call('policySkillsEnabled', env);

/**
 * `CLAUDE_CODE_ENABLE_BACKGROUND_PLUGIN_REFRESH=1` refreshes plugin state at turn boundaries "in
 * non-interactive mode"; off by default.
 */
export const backgroundPluginRefreshEnabled = (
  env: Environment,
  session: { readonly interactive: boolean },
): boolean => call('backgroundPluginRefreshEnabled', env, { interactive: session.interactive });

/** "Accepts a positive whole number in plain digits; anything else is ignored" (default 20). */
export const DEFAULT_MAX_CONCURRENT_SUBAGENTS: number = call('DEFAULT_MAX_CONCURRENT_SUBAGENTS');
export const maxConcurrentSubagents = (env: Environment): number =>
  call('maxConcurrentSubagents', env);

/**
 * `CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS`: "Timeout in milliseconds for cloning or refreshing a plugin
 * marketplace (default: 120000)". A value that is not a positive whole number (digits, `2e3`,
 * `64_000`: spec §3.6.1 item 2, J3.17) leaves the default.
 */
export const DEFAULT_PLUGIN_GIT_TIMEOUT_MS: number = call('DEFAULT_PLUGIN_GIT_TIMEOUT_MS');
export const pluginGitTimeoutMs = (env: Environment): number => call('pluginGitTimeoutMs', env);

/** `CLAUDE_CODE_PLUGIN_PREFER_HTTPS=1`: clone GitHub `owner/repo` shorthand over HTTPS, not SSH. */
export const preferHttps = (env: Environment): boolean => call('preferHttps', env);

/** The wait before the first query in a non-interactive run. */
export interface SyncInstallWait {
  readonly wait: boolean;
  /** No default: without the timeout variable, "synchronous installation waits until complete". */
  readonly timeoutMs?: number;
}

/**
 * `CLAUDE_CODE_SYNC_PLUGIN_INSTALL=1` in non-interactive mode waits for installation;
 * `CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS` bounds the wait. Judgment: an invalid timeout is no bound.
 */
export const syncPluginInstall = (
  env: Environment,
  session: { readonly interactive: boolean },
): SyncInstallWait => call('syncPluginInstall', env, { interactive: session.interactive });

/**
 * Whether telemetry is off. `DISABLE_TELEMETRY`: "any non-empty value ... Setting it to `0` or
 * `false` still opts out". `DO_NOT_TRACK` is "a standard boolean, so `0` leaves telemetry on".
 */
export const telemetryOptedOut = (env: Environment): boolean => call('telemetryOptedOut', env);

/**
 * `CLAUDE_CODE_PLUGIN_SEED_DIR`: "one or more read-only plugin seed directories, separated by `:` on
 * Unix or `;` on Windows"; "Claude Code uses the first seed that contains a given marketplace or
 * plugin cache", so the order is kept. Empty entries are dropped (judgment).
 */
export const pluginSeedDirectories = (
  env: Environment,
  platform: 'posix' | 'windows',
): readonly string[] => call('pluginSeedDirectories', env, platform);

/**
 * `CLAUDE_CODE_TMPDIR`: "Claude Code appends `/claude-{uid}/` to this path on Unix, or `/claude/` on
 * Windows". Without the override the host's default stands ("`/tmp` on macOS, `os.tmpdir()` on Linux
 * and Windows"), which the host passes as `fallback`. The uid is the host's.
 */
export const claudeTempDirectory = (
  env: Environment,
  input: {
    readonly platform: 'posix' | 'windows';
    readonly uid: number;
    readonly fallback: string;
  },
): string => call('claudeTempDirectory', env, input);
