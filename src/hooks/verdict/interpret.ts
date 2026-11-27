import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { RegisteredHandler } from '../contracts.ts';
import type { RawRun } from '../handlers/context.ts';
import type { HandlerVerdict, ParseOutput } from './types.ts';

/**
 * Reads one handler's raw run as the docs read it (hooks.md, "Exit code output", "HTTP response
 * handling", "Timeouts"): one reader per kind of run, a registry over the union so a run the run
 * stage learns to report is a compile error here.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/verdict/interpret.rs`,
 * docs/RUST.md).
 */

const call = coreModule('hooks/verdict/interpret');

/** What the run of `entry` on `event` came to. */
export interface InterpretContext {
  readonly event: HookEvent;
  /** The facts the event fired with. */
  readonly facts: object;
  readonly parse: ParseOutput;
  /** A Monitor call reached the hook through a `Bash` matcher. */
  readonly effectsIgnored?: boolean;
  /** The limit the handler ran under, in milliseconds (`handlerTimeoutMs`); absent for none. */
  readonly timeoutMs?: number | undefined;
  /** The host runs Windows without Git Bash (its default shell is PowerShell). */
  readonly gitBashMissing?: boolean;
  /** `TaskCompleted` fired because a teammate finished its turn. */
  readonly teammateStopped?: boolean;
}

export const interpretRun = (
  entry: RegisteredHandler,
  run: RawRun,
  context: InterpretContext,
): HandlerVerdict => call('interpretRun', entry, run, context);
