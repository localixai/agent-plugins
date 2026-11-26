import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/canonical-json');

/**
 * A JSON text of a value with its object keys sorted by code unit at every depth, so two values
 * equal in content are equal text: the one identity every "same entry", "same source" and "same
 * input" comparison uses. As JSON.stringify writes them, an `undefined` member is left out and an
 * `undefined` array item is `null`; a value with no JSON text (`undefined` itself) is `undefined`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/canonical_json.rs`, docs/RUST.md).
 */
export const canonicalJson = (value: unknown): string => call('canonicalJson', value);
