/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/inputs/web_search.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/inputs/web-search');

/** hooks.md ("WebSearch"): `"standard"` requests a lighter search and `"extended"` the full search. */
export const WEB_SEARCH_MODES: readonly ['standard', 'extended'] = call('WEB_SEARCH_MODES');
export type WebSearchMode = (typeof WEB_SEARCH_MODES)[number];

/**
 * The search a call runs. A `mode` that is neither of the two is read as absent here;
 * `parseToolInput` reports it.
 */
export const effectiveSearchMode = (input: unknown): WebSearchMode =>
  call('effectiveSearchMode', input);
