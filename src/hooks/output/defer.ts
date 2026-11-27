/**
 * Whether a `PreToolUse` `defer` pauses the turn (hooks.md, "Defer a tool call for later"): "Claude
 * Code honors `defer` only in non-interactive mode with the `-p` flag, and only when Claude makes a
 * single tool call in the turn". "In interactive sessions: it logs a warning and ignores the hook
 * result"; with "several tool calls at once: `defer` is ignored with a warning and the tool proceeds
 * through the normal permission flow". The merge says `defer` won; only the host knows the turn.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/defer.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { EventEffects } from './effects.ts';

/** What the host knows of the turn the tool call belongs to. */
export interface DeferTurn {
  /** An interactive terminal session, as `HookSession.interactive`. */
  readonly interactive: boolean;
  /** How many tool calls Claude made in this turn. */
  readonly toolCalls: number;
}

export type DeferResolution =
  /** The merged output holds no `defer`: nothing to resolve. */
  | { readonly kind: 'none' }
  /** The turn ends with `stop_reason: "tool_deferred"` and the pending call stays in the transcript. */
  | { readonly kind: 'deferred' }
  /** `defer` does not apply: log `warning`, run the normal permission flow with no hook decision. */
  | { readonly kind: 'ignored'; readonly warning: PluginIssue };

const call = coreModule('hooks/output/defer');

/** The resolution of the merged `effects` of a `PreToolUse` dispatch in `turn`. */
export const resolveDefer = (
  effects: Pick<EventEffects, 'permissionDecision'>,
  turn: DeferTurn,
): DeferResolution => call('resolveDefer', effects, turn);
