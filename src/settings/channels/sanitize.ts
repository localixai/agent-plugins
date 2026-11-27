/**
 * How Claude Code prepares `description` and `input_preview` before relaying a permission request
 * (channels-reference.md, "Permission request fields"). Pure text rules.
 *
 * Judgments (spec/judgments.md): each neutralized character becomes U+FFFD; the 3,500 code points are
 * split evenly, the start and the end 1,750 each, around the marker; an array of more than 10,000
 * items counts as "extremely large"; the credential patterns are the common provider prefixes.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/channels/sanitize.rs`, docs/RUST.md);
 * `inputPreview` stays here: it serializes the host's tool input, which may hold a cycle or a
 * `bigint` no boundary can copy (docs/RUST.md, "What stays TypeScript"), and asks the core for the
 * text rules.
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/channels/sanitize');

const UNSERIALIZABLE = '(value unserializable)';
const MAX_ARRAY = 10_000;

/** `[REDACTED]` "in place of a recognizable provider credential token". */
export const maskCredentials = (text: string): string => call('maskCredentials', text);

/** "For a longer value, you receive its start and its end around a counted `⋯ N code points elided ⋯` marker." */
export const elide = (text: string): string => call('elide', text);

/** Mask, neutralize, fold each run of whitespace to one space, then elide. */
export const sanitizeRelayText = (text: string): string => call('sanitizeRelayText', text);

const serializable = (value: unknown): string | undefined => {
  if (Array.isArray(value) && value.length > MAX_ARRAY) return undefined;
  try {
    return JSON.stringify(value) ?? undefined;
  } catch {
    return undefined;
  }
};

const fieldText = (value: unknown): string => {
  if (typeof value === 'string') return JSON.stringify(sanitizeRelayText(value));
  const json = serializable(value);
  return json === undefined ? UNSERIALIZABLE : sanitizeRelayText(json);
};

/**
 * The `input_preview`: the arguments "as JSON-shaped display text, keyed per top-level field". The limit
 * applies to each field separately and a string keeps its own quotes; a field that cannot be
 * serialized shows `(value unserializable)` and keeps its key; key names are masked too.
 */
export function inputPreview(input: Readonly<Record<string, unknown>>): string {
  const fields = Object.entries(input).map(
    ([key, value]) => `${JSON.stringify(maskCredentials(key))}: ${fieldText(value)}`,
  );
  return `{${fields.join(', ')}}`;
}

/** "When the model gives no description, the field is the constant `Run shell command`." */
export const relayDescription = (description: string | undefined): string =>
  call('relayDescription', description);
