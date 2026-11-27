/**
 * How long a Bash or PowerShell command runs, in the foreground and in the background
 * (tools-reference.md, "Timeout and output limits", "Time limit for background commands").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/bash_limits.rs`,
 * docs/RUST.md).
 */
import type { Environment } from '../../../ports/environment.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/bash-limits');

/**
 * How long a foreground Bash or PowerShell command runs (tools-reference.md, "Timeout and output
 * limits"; hooks.md: "Values above the maximum are reduced to the maximum rather than rejected").
 * `BASH_DEFAULT_TIMEOUT_MS` is the default when Claude passes no timeout, two minutes out of the
 * box; `BASH_MAX_TIMEOUT_MS` sets the ceiling, ten minutes out of the box, and the effective ceiling
 * is the larger of the two. A timeout that is not a positive number counts as none, as does a
 * variable that is not a positive integer (spec/judgments.md).
 */
export const effectiveBashTimeout = (requested: unknown, env: Pick<Environment, 'get'>): number =>
  call('effectiveBashTimeout', requested, env);

/** What the session is, as far as the background time limit goes. */
export interface BackgroundSession {
  /** A run with `-p`, an Agent SDK application, a CI job or a cloud session; a local session has no limit. */
  readonly unattended: boolean;
  /** A `-p` run whose prompt is text, not `--input-format stream-json`: its defaults are 10 minutes. */
  readonly printWithTextPrompt: boolean;
}

export interface BackgroundRequest {
  readonly session: BackgroundSession;
  readonly env: Pick<Environment, 'get'>;
  /** The `timeout` Claude passed with `run_in_background`. */
  readonly timeout?: number;
  /** A foreground command that reached its timeout and moved to the background. */
  readonly moved?: boolean;
}

/**
 * How long a background command may run, or `undefined` for none (tools-reference.md, "Time limit
 * for background commands"). A command Claude starts in the background gets 30 minutes, or its
 * `timeout` up to 2 hours; one that moves from the foreground gets 30 minutes from the move.
 * `BASH_DEFAULT_TIMEOUT_MS` above the default replaces it (a lower value never shortens it), and an
 * effective ceiling above 2 hours raises the maximum.
 */
export const backgroundTimeLimit = (request: BackgroundRequest): number | undefined =>
  call('backgroundTimeLimit', request);
