import { coreModule } from '../../wasm/bridge.ts';

/**
 * `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` (env-vars.md, "Variables" and "What the subprocess environment
 * scrub removes"): with it on, "Claude Code removes credentials from the environments of the
 * subprocesses it starts, such as Bash commands, hooks, and stdio MCP servers" and keeps them for
 * its own API calls. The table (`format/credential-env`, `scrubAction`) decides per variable; this
 * file applies it to a snapshot, pure.
 *
 * Host duties the kit cannot do (L3): on Linux the scrub "also runs Bash subprocesses in an isolated
 * PID namespace so they can't read host process environments through `/proc`" (a process option,
 * not an environment edit), and `claude-code-action` sets the variable itself when
 * `allowed_non_write_users` is configured.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/subprocess_scrub.rs`,
 * docs/RUST.md).
 */
const call = coreModule('loading/resolve/subprocess-scrub');

/** Whether the snapshot turns the scrub on: an on/off variable (`format/env-flag`). */
export const subprocessScrubEnabled = (snapshot: Readonly<Record<string, string>>): boolean =>
  call('subprocessScrubEnabled', snapshot);

export interface ScrubOptions {
  /** The Claude Code release whose behaviour to follow, such as `2.1.319`; absent is the current table. */
  readonly version?: string;
}

/**
 * The environment a subprocess gets: `snapshot` with the credentials the scrub removes taken out and
 * the userinfo cut from registry URLs, when `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` is on; the snapshot
 * unchanged (a copy) when it is not. A secret whose name and value look like no credential stays:
 * "use it as one layer alongside narrow permission rules".
 */
export const scrubEnvironment = (
  snapshot: Readonly<Record<string, string>>,
  options: ScrubOptions = {},
): Record<string, string> => call('scrubEnvironment', snapshot, options);
