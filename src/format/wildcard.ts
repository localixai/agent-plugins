/**
 * The kit's one `*` wildcard: `*` is any run of characters (none included), every other character
 * is literal and the whole text must match. The settings URL patterns (`allowedHttpHookUrls`, the
 * MCP `serverUrl` entries) match with `matchesWildcard`; a pattern that needs flags or a narrower
 * `*` (a rule's tool name, a path segment, a domain label) compiles `wildcardSource`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/wildcard.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/wildcard');

/** `text` as regular-expression source that matches only itself. */
export const escapeRegExp = (text: string): string => call('escapeRegExp', text);

/** A `*` pattern as regular-expression source, `star` what a `*` matches (any text by default). */
export const wildcardSource = (pattern: string, star?: string): string =>
  call('wildcardSource', pattern, star);

/**
 * Whether `text` matches `pattern` whole, each `*` any run of characters. Linear, with no regular
 * expression; a caller that matches without case folds both sides first.
 */
export const matchesWildcard = (pattern: string, text: string): boolean =>
  call('matchesWildcard', pattern, text);
