import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/defined');

/**
 * An object without its `undefined` values, so it can be spread over another without erasing what
 * the other holds, and satisfies `exactOptionalPropertyTypes` (an optional key is absent, never
 * `undefined`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/defined.rs`, docs/RUST.md).
 */
export const defined = <T extends object>(
  value: T,
): { [K in keyof T]?: Exclude<T[K], undefined> } => call('defined', value);
