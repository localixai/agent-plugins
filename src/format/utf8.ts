/**
 * Sizes in UTF-8 bytes, for every limit the docs give in bytes: one byte count and one cut that
 * never splits a character.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/utf8.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/utf8');

/** The UTF-8 bytes of `text`. */
export const utf8Length = (text: string): number => call('utf8Length', text);

/** The longest prefix of `text` of at most `limit` UTF-8 bytes that ends between two characters. */
export const prefixWithinBytes = (text: string, limit: number): string =>
  call('prefixWithinBytes', text, limit);
