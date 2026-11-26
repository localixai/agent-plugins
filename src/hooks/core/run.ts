import type { CoreHookEvent } from '../../format/core/hooks.ts';
import type { CoreProcess } from '../../format/core/subprocess.ts';
import type { CommandRunner } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { CoreRun } from './types.ts';

/**
 * One invocation of a core handler (core §7.3.3; Agent Plugins Extended §18.1.2, "Failures, limits
 * and presentation"): a fresh process per invocation through the `CommandRunner` port, in exec form
 * (`command` one token, `args` separate, no shell), the plugin root as working directory, the core
 * input as one JSON object on stdin, stderr kept apart from stdout so it cannot make the output
 * invalid. The time limit is a Claude Code `command` hook's default on the same event (J18.5): 30
 * seconds on `UserPromptSubmit`, 600 seconds on the other four.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/core/run.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/core/run');

export interface CoreInvocation {
  readonly handlerId: string;
  readonly pluginId: string;
  readonly event: CoreHookEvent;
  readonly process: CoreProcess;
  /** The handler as written in `hooks.json`: its `command` and `args` joined by spaces (J18.22). */
  readonly label: string;
  readonly input: Readonly<Record<string, unknown>>;
}

/** Runs the handler once and reads what it came to; the port's own failure is a failed run too. */
export const runCoreHandler = (
  runner: CommandRunner,
  invocation: CoreInvocation,
  signal: AbortSignal,
): Promise<CoreRun> => call.async('runCoreHandler', runner, invocation, signal);
