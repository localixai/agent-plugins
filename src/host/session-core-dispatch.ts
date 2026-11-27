import type {
  HookOutcome,
  HookPorts,
  HookSession,
  HookSourceKind,
  StopBlockCap,
} from '../hooks/index.ts';
import type { CoreCallFacts } from '../hooks/core/types.ts';
import type { HookEvent } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { HostState } from './state.ts';

/**
 * One occurrence for both forms (Agent Plugins Extended §18.1.2, "When each side runs"): with no
 * core handler on it the dispatch is Claude Code's alone, exactly as without core packages
 * (principle 1). `PreToolUse` runs Claude Code's handlers first and the core's on the input the call
 * would now run with, a denial included (J18.1); every other shared event runs both sides together
 * and waits for both (J18.2). On `Stop` the cap counts the combined continuation once (J18.9).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_core_dispatch.rs`, docs/RUST.md).
 */

/** What the core side of a session's dispatch reads (the core's `host/session_core_hooks.rs`). */
export interface CoreHandlerUse {
  readonly state: HostState;
  readonly session: HookSession;
  readonly ports: HookPorts;
  /** The base environment of a plugin hook's process in this session (`subprocessEnvironment('hook')`). */
  readonly base: () => Readonly<Record<string, string>>;
}

/** How a session runs its Claude Code side: with the Stop cap applied, or without (the caller applies it). */
export type ClaudeSide<E extends HookEvent> = (capped: boolean) => Promise<HookOutcome<E>>;

export interface CoreDispatch<E extends HookEvent> {
  readonly event: E;
  readonly facts: object;
  readonly use: CoreHandlerUse;
  readonly context: { readonly cwd: string; readonly subagent: boolean };
  readonly core: CoreCallFacts | undefined;
  readonly sourcesOnly: readonly HookSourceKind[] | undefined;
  readonly stopCap: StopBlockCap;
  readonly signal: AbortSignal;
}

const call = coreModule('host/session-core-dispatch');

/** The state crosses as the core's handle of it, not the host's object. */
export const dispatchBothForms = <E extends HookEvent>(
  request: CoreDispatch<E>,
  claude: ClaudeSide<E>,
): Promise<HookOutcome<E>> =>
  call.async(
    'dispatchBothForms',
    { ...request, use: { ...request.use, state: request.use.state.handle } },
    claude,
  );
