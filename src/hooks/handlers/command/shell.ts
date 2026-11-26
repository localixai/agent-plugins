/**
 * The shell a command hook runs in, or none: "A command hook runs as exec form when `args` is set",
 * and exec form has no shell; shell form runs in the hook's `shell`, else the host's default
 * (hooks.md, "Exec form and shell form"; "Command hook fields", `shell`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/command/shell.rs`, docs/RUST.md).
 */
import type { ShellKind } from '../../../ports/index.ts';
import { coreModule } from '../../../wasm/bridge.ts';
import type { HandlerEntry, RunContext } from '../context.ts';

const call = coreModule('hooks/handlers/command/shell');

export const shellOf = (
  entry: HandlerEntry<'command'>,
  context: Pick<RunContext, 'defaultShell'>,
): ShellKind | undefined => call('shellOf', entry, context);
