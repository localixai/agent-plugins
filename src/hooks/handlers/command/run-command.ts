import { coreModule } from '../../../wasm/bridge.ts';
import type { HandlerEntry, HandlerResult, RunContext } from '../context.ts';

/**
 * A `command` hook (hooks.md, "Command hook fields"): the form it runs in, the process it gets,
 * and the run the port reports. What the exit code means is the interpreter's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/command/run_command.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/handlers/command/run-command');

export const runCommand = (
  entry: HandlerEntry<'command'>,
  context: RunContext,
): Promise<HandlerResult> => call.async('runCommand', entry, context);
