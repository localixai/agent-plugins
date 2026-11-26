/**
 * What each event honours of a hook's JSON output and what it discards (hooks.md, "JSON output":
 * "Every event accepts them, but some events discard them ... Each event's section says so"). One
 * row per event of the official union.
 *
 * `terminalSequence` is dropped nowhere here: "Claude Code writes the sequence itself when it
 * processes your hook's output, so the field works on events that discard `systemMessage` and
 * `continue`", and its two exceptions (non-interactive sessions, a command `WorktreeCreate` hook)
 * are not a matter of the event's output but of the host and handler.
 * Judgment (spec/judgments.md): that includes the events that "discard their JSON output fields".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/caps.rs`, docs/RUST.md).
 */
import type { HookEvent } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/caps');

/** The groups of fields an event can discard. */
export const CAPPED_FIELDS: readonly [
  'continue',
  'systemMessage',
  'terminalSequence',
  'decision',
  'specific',
] = call('CAPPED_FIELDS');
export type CappedField = (typeof CAPPED_FIELDS)[number];

export interface OutputCap {
  readonly accepts: ReadonlySet<CappedField>;
  readonly discards: ReadonlySet<CappedField>;
}

export const OUTPUT_CAPS: { readonly [E in HookEvent]: OutputCap } = call('OUTPUT_CAPS');
