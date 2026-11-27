/**
 * A foreground command that reaches its timeout, moved to the background, and the notices of a
 * background command (tools-reference.md, "Foreground commands that move to the background").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/bash_background.rs`,
 * docs/RUST.md).
 */
import type { Environment } from '../../../ports/environment.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/bash-background');

/** What the session is, as far as moving a timed-out command goes. */
export interface MoveSession {
  /** Running in bare mode (headless.md, "Start faster with bare mode"). */
  readonly bare: boolean;
}

/**
 * Whether a foreground command that reaches its timeout moves to the background instead of
 * stopping (tools-reference.md, "Foreground commands that move to the background"): it does, unless
 * the command starts with `sleep`, `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1` is set (env-vars.md:
 * "Set to `1`") or the session runs in bare mode. A line "starts with `sleep`" when its first word
 * is `sleep` (spec/judgments.md).
 */
export const shouldMoveToBackground = (
  command: string,
  env: Pick<Environment, 'get'>,
  session: MoveSession,
): boolean => call('shouldMoveToBackground', command, env, session);

/** "`Command did not complete within its 120s timeout and was moved to the background`", seconds of the timeout. */
export const movedByTimeoutNotice = (timeoutMs: number): string =>
  call('movedByTimeoutNotice', timeoutMs);

/**
 * The note of a moved command that changes directory: "A `cd`, `pushd`, `popd`, or `chdir` inside a
 * command that is moved to the background never carries over: the result states `Session cwd
 * remains <dir>; directory changes made by the backgrounded command do not apply to subsequent
 * commands.`" `undefined` when no subcommand starts with one of the four.
 */
export const backgroundCwdNote = (command: string, cwd: string): string | undefined =>
  call('backgroundCwdNote', command, cwd);

/** "`Background command "<description>" was stopped after reaching its background time limit`". */
export const backgroundStopNotice = (description: string): string =>
  call('backgroundStopNotice', description);
