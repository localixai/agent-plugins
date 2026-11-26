/**
 * Judgment (spec/judgments.md): the docs count tokens (the skill listing budget, re-attached skill
 * content, MCP output limits) and name no tokenizer, so wherever the kit counts tokens without one
 * it takes four characters to a token, a character being a code point, as the skill listing
 * counts characters. A host with a tokenizer passes its own counter where one is taken.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/token_estimate.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/token-estimate');

/** Characters (code points) to one token. */
export const CHARS_PER_TOKEN: number = call('CHARS_PER_TOKEN');

/** The tokens of `text` by that estimate, rounded up. */
export const estimateTokens = (text: string): number => call('estimateTokens', text);
