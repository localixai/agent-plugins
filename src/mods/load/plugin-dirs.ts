import { isFalsyEnv, isTruthyEnv } from '../../format/env-flag.ts';
import { isAbsolutePath } from '../../format/path-text.ts';
import { MOD_SETTINGS } from '../../protocol/mods-settings.ts';
import { sideloadVerdict, type SideloadVerdict } from '../../settings/managed/index.ts';
import { createPolicyView } from '../../settings/view.ts';
import type { ModsPolicyInput } from './policy-input.ts';

/**
 * Plugin directories loaded for a session (plugins/mods/reference.md, "Settings and environment
 * variables" and "Commands"): `--plugin-dir`, `CLAUDE_CODE_PLUGIN_DIRS` and
 * `CLAUDE_CODE_PLUGIN_DIR_WATCH`.
 */

const SEPARATOR = { posix: ':', windows: ';' } as const;

/**
 * `CLAUDE_CODE_PLUGIN_DIRS`: "Absolute paths separated by `:`, or `;` on Windows". An empty piece is
 * left out (J3.18), and so is a relative one: each entry is absolute or starts with `~` (spec §3.6.1).
 */
export function parsePluginDirs(
  value: string | undefined,
  platform: 'posix' | 'windows',
): readonly string[] {
  if (value === undefined) return [];
  return value
    .split(SEPARATOR[platform])
    .map((piece) => piece.trim())
    .filter((piece) => piece.startsWith('~') || isAbsolutePath(piece));
}

/**
 * The directories `CLAUDE_CODE_PLUGIN_DIRS` names: the variable is read from the process environment,
 * else from `env` in `~/.claude/settings.json` (its homes in `MOD_SETTINGS`).
 */
function variableDirs(policy: ModsPolicyInput): readonly string[] {
  const homes: readonly string[] = MOD_SETTINGS.CLAUDE_CODE_PLUGIN_DIRS.homes;
  const fromEnvironment = homes.includes('environment')
    ? policy.env.CLAUDE_CODE_PLUGIN_DIRS
    : undefined;
  const fromSettings = homes.includes('user-env')
    ? policy.user.env?.CLAUDE_CODE_PLUGIN_DIRS
    : undefined;
  return parsePluginDirs(fromEnvironment ?? fromSettings, policy.platform);
}

/**
 * What managed `disableSideloadFlags` does with the session's flags and the variable: the one verdict
 * of `settings/managed/sideload.ts`, read from the managed file of the policy (which alone carries
 * the key, `MOD_SETTINGS`).
 */
function verdictOf(policy: ModsPolicyInput): SideloadVerdict {
  const view = createPolicyView([{ scope: 'managed', settings: policy.managed ?? {} }]);
  return sideloadVerdict(view, {
    pluginDirs: policy.flags.pluginDirs,
    pluginUrls: policy.flags.pluginUrls ?? [],
    agents: policy.flags.agents ?? false,
    mcpServers: policy.flags.mcpServers ?? [],
    pluginDirsVariable: variableDirs(policy),
    cloud: policy.cloud ?? false,
  });
}

/**
 * Every directory the session loads "as `--plugin-dir` does": the flags, then the variable. A
 * directory named twice loads once. When managed `disableSideloadFlags` makes startup exit
 * (`sideloadRefusal`), nothing loads.
 */
export function pluginDirsOf(policy: ModsPolicyInput): readonly string[] {
  if (verdictOf(policy).refusals.length > 0) return [];
  return [...new Set([...policy.flags.pluginDirs, ...variableDirs(policy)])];
}

/**
 * The error startup exits with when managed `disableSideloadFlags` is set and a plugin, agent or MCP
 * server was sideloaded by flag, or `CLAUDE_CODE_PLUGIN_DIRS` names a plugin folder: one line per
 * rejected flag or variable, from `sideloadVerdict`. `--plugin-dir` has the documented message
 * (admin.md, "Test what users see"); the others are judgments recorded beside it.
 */
export function sideloadRefusal(policy: ModsPolicyInput): string | undefined {
  const { refusals } = verdictOf(policy);
  return refusals.length === 0 ? undefined : refusals.join('\n');
}

/** Where a mod that reloads on save was loaded from: `--plugin-dir`, or a skills-directory plugin (§4.5.1). */
export type ReloadSource = 'plugin-dir' | 'skills-dir';

/** The facts of a session that decide whether a mod reloads on save (§10.8.2). */
export interface ReloadSession {
  /** An interactive session in the terminal (not the Desktop app or an IDE). */
  readonly isInteractiveTerminal: boolean;
  /** The session takes more than one prompt, a non-interactive one included. */
  readonly takesMorePrompts: boolean;
}

/** Whether a mod of each source reloads on save while the variable is unset (§10.8.2 item 1, 2.1.330). */
const RELOADS_BY_DEFAULT: Readonly<Record<ReloadSource, (session: ReloadSession) => boolean>> = {
  'plugin-dir': (session) => session.isInteractiveTerminal,
  'skills-dir': (session) => session.takesMorePrompts,
};

/**
 * Whether a session reloads a mod when one of its files is saved (§10.8.2 item 1, §10.12.2):
 * unset, a skills-directory mod reloads in any session that takes more than one prompt and a
 * `--plugin-dir` mod only in an interactive terminal session; `CLAUDE_CODE_PLUGIN_DIR_WATCH` on
 * makes `--plugin-dir` mods reload as skills-directory mods do, off turns reloading off everywhere.
 */
export function reloadsOnSave(
  policy: ModsPolicyInput,
  source: ReloadSource,
  session: ReloadSession,
): boolean {
  // An on/off variable: unset or another value leaves the default (spec §3.6.1, J3.16).
  const value = policy.env.CLAUDE_CODE_PLUGIN_DIR_WATCH;
  if (isFalsyEnv(value)) return false;
  if (isTruthyEnv(value)) return RELOADS_BY_DEFAULT['skills-dir'](session);
  return RELOADS_BY_DEFAULT[source](session);
}
