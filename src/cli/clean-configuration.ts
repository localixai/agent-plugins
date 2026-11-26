/**
 * The recipe of debug-your-config.md, "Test against a clean configuration": an empty directory, a
 * fresh `CLAUDE_CONFIG_DIR`, and `--setting-sources user`, so that the session has none of your own
 * configuration; then what each outcome of reproducing the problem points to. Pure: it only builds
 * the launch and the advice, the host or the person runs them.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/clean_configuration.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/clean-configuration');

export interface CleanLaunch {
  /** The empty directory to run in. */
  readonly directory: string;
  /** The fresh configuration directory. */
  readonly configDir: string;
  /** The variable that "applies to this one launch" (or, in PowerShell, to the rest of the shell session). */
  readonly env: { readonly CLAUDE_CONFIG_DIR: string };
  /** `--setting-sources user` turns off project configuration (`.claude/settings.json`, `CLAUDE.md`, `.mcp.json`). */
  readonly args: readonly ['--setting-sources', 'user'];
}

/** The launch for a home directory: `~/claude-clean/empty` and `~/claude-clean/config`. */
export const cleanLaunch = (home: string): CleanLaunch => call('cleanLaunch', home);

/** The documented commands, per shell. */
export type CleanShell = 'posix' | 'powershell';

export const cleanLaunchCommands = (
  shell: CleanShell,
): {
  readonly create: string;
  readonly launch: string;
} => call('cleanLaunchCommands', shell);

/** What reproducing the problem in the clean session means. */
export type CleanOutcome = 'problem-gone' | 'problem-persists';

/**
 * "Problem gone": the cause is in your user or project configuration, so copy `~/.claude.json` and the
 * files in `~/.claude` into the clean directory one at a time, then try the project without
 * `--setting-sources user`. "Problem persists": the cause is outside it; check `/status` for managed
 * settings and the shell for environment variables.
 */
export const afterCleanRun = (
  outcome: CleanOutcome,
): {
  readonly cause: 'user-or-project-configuration' | 'outside-user-and-project-configuration';
  readonly next: readonly string[];
} => call('afterCleanRun', outcome);
