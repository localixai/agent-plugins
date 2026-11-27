/**
 * Hook handlers (hooks.md, "Hook handler fields"): the five types and the fields each accepts,
 * bound to the official `HookHandlerOf<T>` so an SDK update that adds a field fails the typecheck
 * here. The common fields apply to every type; the rest are the type's own.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/hooks_handlers.rs`, docs/RUST.md).
 */
import type { HookHandler } from '../protocol/hook-config.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

const call = coreModule('format/hooks-handlers');

export interface HandlerRead {
  /** Undefined when the handler has an error. */
  readonly handler: HookHandler | undefined;
  readonly issues: readonly PluginIssue[];
}

/** Reads one handler object at `path` (`hooks.PreToolUse[0].hooks[1]`). */
export const readHandler = (raw: unknown, path: string): HandlerRead =>
  call('readHandler', raw, path);
