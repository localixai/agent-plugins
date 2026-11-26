/**
 * Implemented by the Rust core (`crates/agent-plugins/src/protocol/effort.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { EffortLevel, Settings } from './official.ts';
import type { Assert, Equal } from './type-equality.ts';

const call = coreModule('protocol/effort');

/**
 * The named reasoning-effort levels, lowest first (skills and agents `effort` frontmatter, the
 * `effortLevel` setting, `CLAUDE_CODE_EFFORT_LEVEL`). Asserted equal to the SDK's `EffortLevel`:
 * a level Anthropic adds fails the typecheck here.
 */
export const EFFORT_LEVELS: readonly ['low', 'medium', 'high', 'xhigh', 'max'] =
  call('EFFORT_LEVELS');

/** @public A compile-time drift guard: the typecheck fails when the SDK changes the union. */
export type EffortLevelsMatchSdk = Assert<Equal<(typeof EFFORT_LEVELS)[number], EffortLevel>>;

/** The level a settings file may save (`effortLevel`, `modelSettings.<model>.effortLevel`). */
export type SavedEffortLevel = NonNullable<Settings['effortLevel']>;

/**
 * The levels a file may save: `max` is "session-only and is not written" (settings-reference.md,
 * `effortLevel` and `modelSettings` list low, medium, high and xhigh; `max` is valid only for
 * `maxEffortLevel`). Asserted equal to the SDK's `Settings['effortLevel']`.
 */
export const SAVED_EFFORT_LEVELS: readonly ['low', 'medium', 'high', 'xhigh'] =
  call('SAVED_EFFORT_LEVELS');

/** @public A compile-time drift guard: the typecheck fails when the SDK changes the union. */
export type SavedEffortLevelsMatchSdk = Assert<
  Equal<(typeof SAVED_EFFORT_LEVELS)[number], SavedEffortLevel>
>;
