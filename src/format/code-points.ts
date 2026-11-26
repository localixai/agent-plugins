import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/code-points');

/**
 * The length of a text in Unicode code points, the unit the specification counts a name or an
 * option label in (spec §3.4.2, §4.3): a character outside the Basic Multilingual Plane is one,
 * not the two UTF-16 units `String.length` gives.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/code_points.rs`, docs/RUST.md).
 */
export const codePointLength = (text: string): number => call('codePointLength', text);
