/**
 * How the validator names the kind of a JSON value in its messages. Zod's wording
 * ("expected string, received number") uses the bare name; the hooks messages use an article
 * ("received a number").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/received.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/validate/received');

export const kindOf = (value: unknown): string => call('kindOf', value);
