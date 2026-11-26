import type { PortFailure } from '../../../ports/index.ts';
import type { HookHandlerType } from '../../../protocol/hook-config.ts';
import { coreModule } from '../../../wasm/bridge.ts';
import type { RegisteredHandler } from '../../contracts.ts';
import type { HandlerResult, RawRun } from '../context.ts';

const call = coreModule('hooks/handlers/remote/failure-run');

/**
 * What a port's `PortFailure` is to a hook run. "Timeout: the hook is canceled, as described under
 * Timeouts", so a `timeout` is the shared `timed-out` run; an `aborted` request is a session abort,
 * the shared `cancelled` run. Any other failure is the type's own non-blocking failure, which the
 * caller names.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/remote/failure_run.rs`,
 * docs/RUST.md).
 */
export const failureRun = (failure: PortFailure, other: (message: string) => RawRun): RawRun =>
  call('failureRun', failure, other);

/**
 * A hook whose port the host did not give does not run: "a missing port is a `PluginIssue` for the
 * component that needed it" (docs/ARCHITECTURE.md, "Ports").
 */
export const withoutPort = (
  entry: RegisteredHandler,
  type: HookHandlerType,
  port: string,
): HandlerResult => call('withoutPort', entry, type, port);
