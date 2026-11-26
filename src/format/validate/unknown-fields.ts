import { coreModule } from '../../wasm/bridge.ts';
import type { Finding } from './finding.ts';

/**
 * What `claude plugin validate` says about a key it does not know (oracle 2.1.319). Claude Code
 * ignores such a key at load time; the validator tells the author why it is probably there:
 * a field of the marketplace entry, another tool's manifest, or a misspelling.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/unknown_fields.rs`,
 * docs/RUST.md).
 */
const call = coreModule('format/validate/unknown-fields');

/** Where the key was found: only a manifest is told that a key belongs in the marketplace entry. */
export const FIELD_SITES: readonly ['manifest', 'other'] = call('FIELD_SITES');
export type FieldSite = (typeof FIELD_SITES)[number];

/** The warning for a key `known` does not list, at `path`. */
export const unknownFieldFinding = (
  key: string,
  known: readonly string[],
  path: string = key,
  site: FieldSite = 'manifest',
): Finding => call('unknownFieldFinding', key, known, path, site);
