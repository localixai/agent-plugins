import type { CoreHookEvent } from '../../format/core/hooks.ts';
import type { CommandResult } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { CoreResult } from './types.ts';

/**
 * What a core handler's process came to (core §7.3.2 item 5, §7.3.3): a status other than `0`, a
 * timeout or a process that never started is a failure; on status `0`, no bytes or only JSON
 * whitespace is no opinion, and anything but exactly one JSON object of one of the event's closed
 * output shapes is a failure, `{}` included. Exit status 2 is a failure like any other (Agent
 * Plugins Extended §18.1.2, "Failures": never a block), and plain text is never context.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/core/output.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/core/output');

export type CoreReading = { readonly result?: CoreResult } | { readonly failure: string };

/** Reads one finished run of a core handler for its event. */
export const readCoreRun = (
  event: CoreHookEvent,
  run: CommandResult,
  timeoutMs: number,
): CoreReading => call('readCoreRun', event, run, timeoutMs);
