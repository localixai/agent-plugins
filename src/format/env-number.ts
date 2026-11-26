/**
 * A numeric environment variable (spec §3.6.1 item 2, J3.17): plain digits, scientific notation
 * (`2e3` is 2000) and digit separators (`64_000` is 64000), unless the variable's row says plain
 * digits only. A sign, a blank or any other character makes the value unreadable (judgment). A value that cannot be read is `undefined`, so the caller
 * keeps its default.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/env_number.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/env-number');

/** The number a numeric variable holds, or `undefined` when it holds none. */
export const readNumberEnv = (text: string | undefined): number | undefined =>
  call('readNumberEnv', text);

/** A whole number, `0` included, read as `readNumberEnv` reads it. */
export const wholeEnv = (text: string | undefined): number | undefined => call('wholeEnv', text);

/** A whole number above zero, read as `readNumberEnv` reads it. */
export const positiveWholeEnv = (text: string | undefined): number | undefined =>
  call('positiveWholeEnv', text);

/** A whole number above zero in plain digits only, for a variable whose row says so. */
export const plainPositiveWholeEnv = (text: string | undefined): number | undefined =>
  call('plainPositiveWholeEnv', text);
