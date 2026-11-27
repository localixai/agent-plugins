import { coreModule } from '../../wasm/bridge.ts';
import type { Finding } from './finding.ts';

/**
 * The type rules of `plugin.json` as `claude plugin validate` applies them (oracle 2.1.319): one
 * check per key, in the order the validator reports them. A key with no entry here is either a
 * path key (`plugin-paths.ts`), a listing field the validator reads no further, or unknown.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/plugin_fields.rs`, docs/RUST.md).
 */

const call = coreModule('format/validate/plugin-fields');

/** Every key a manifest may carry without an unknown-field warning. */
export const PLUGIN_KEYS: readonly string[] = call('PLUGIN_KEYS');

/** The keys a misspelt one is compared with: those the validator reads, not those it only lets through. */
export const SUGGESTED_KEYS: readonly string[] = call('SUGGESTED_KEYS');

/** The type errors of the keys present, in the validator's order. */
export const fieldFindings = (manifest: Readonly<Record<string, unknown>>): Finding[] =>
  call('fieldFindings', manifest);

/** `name` is the one key a manifest cannot do without. */
export const missingName = (manifest: Readonly<Record<string, unknown>>): Finding[] =>
  call('missingName', manifest);

/** The free-form `metadata` and the `experimental` block: a wrong type is only a warning. */
export const looseObjectFindings = (manifest: Readonly<Record<string, unknown>>): Finding[] =>
  call('looseObjectFindings', manifest);
