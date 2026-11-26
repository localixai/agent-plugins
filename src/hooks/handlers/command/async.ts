import type { CommandResult, CommandRun, CommandRunner } from '../../../ports/index.ts';
import { coreModule } from '../../../wasm/bridge.ts';
import type { BackgroundResult, ForegroundRun } from '../context.ts';

/**
 * Background command hooks (hooks.md, "Run hooks in the background" and "Command hook fields").
 * `async: true` "runs in the background without blocking"; `asyncRewake: true` "runs in the
 * background and wakes Claude on exit code 2", and "implies async" (the SDK type). The timeout is
 * not enforced on an `async` hook but is on an `asyncRewake` one: that is decided where the
 * limit is computed (`handlerTimeoutMs`), so the request here already carries it or not.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/command/async.rs`,
 * docs/RUST.md).
 */

const call = coreModule('hooks/handlers/command/async');

export type RunMode = 'foreground' | 'async' | 'rewake';

interface Mode {
  /** The engine does not wait for the process. */
  readonly background: boolean;
  /** An exit with code 2 wakes Claude. */
  readonly wakes: boolean;
}

export const MODES: { readonly [M in RunMode]: Mode } = call('MODES');

export const modeOf = (handler: {
  readonly async?: boolean;
  readonly asyncRewake?: boolean;
}): RunMode => call('modeOf', handler);

/**
 * What a runner answers when it can tell that a process the hook started still holds the output
 * open: "Claude Code finishes reading the hook's output shortly after the hook's own process
 * exits", and such a capture is suspect (docs/API.md, "Ports"). The `CommandRunner` port does not
 * type the field (`src/ports` is not this stream's), so a runner declares its answer as this type
 * and the compiler checks the name.
 */
export interface HeldOpenResult extends CommandResult {
  readonly outputHeldOpen?: boolean;
}

/**
 * The port's answer as the run the interpreter reads. A start error under an aborted signal is a
 * cancelled run (Node's `spawn({ signal })` answers an abort with an error, not an exit), and a
 * signal aborted before the start runs nothing; a run that exited keeps its exit.
 */
export const settle = (runner: CommandRunner, request: CommandRun): Promise<ForegroundRun> =>
  call.async('settle', runner, request);

/** Starts the process and returns at once; `done` settles when it ends. */
export const startInBackground = (
  runner: CommandRunner,
  request: CommandRun,
  mode: Mode,
): { readonly kind: 'background'; readonly done: Promise<BackgroundResult> } =>
  call('startInBackground', runner, request, mode);
