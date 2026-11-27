/**
 * Implemented by the Rust core (`crates/agent-plugins/src/protocol/exit_reasons.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { ExitReason } from './official.ts';
import type { Assert, Equal } from './type-equality.ts';

const call = coreModule('protocol/exit-reasons');

/**
 * Why a session ends (mods reference.md, "Session": `e.reason` of `session.end`; `/branch` reports
 * `resume`), tied to the official `ExitReason` union: an SDK update that adds or drops a reason
 * fails the typecheck here.
 */
export const SESSION_END_REASONS: readonly [
  'clear',
  'resume',
  'logout',
  'prompt_input_exit',
  'other',
] = call('SESSION_END_REASONS');

/** One of `SESSION_END_REASONS`: the `reason` of `session.end`. */
export type SessionEndReason = (typeof SESSION_END_REASONS)[number];

/** @public A compile-time drift guard: the typecheck fails when the SDK changes the union. */
export type SessionEndReasonsMatchSdk = Assert<
  Equal<(typeof SESSION_END_REASONS)[number], ExitReason>
>;
