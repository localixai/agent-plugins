import { coreModule } from '../../wasm/bridge.ts';
import type { NameContext } from '../names.ts';
import type { Finding } from './finding.ts';

/**
 * The checks of `claude plugin validate` on a manifest's `name` (oracle 2.1.319): an empty name, a
 * space, a control or bidirectional-formatting character are errors and the only report; any
 * other name gets the reserved-name and kebab-case advice, which comes after the schema passes.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/plugin_name.rs`,
 * docs/RUST.md).
 */
const call = coreModule('format/validate/plugin-name');

export interface NameOptions {
  /** An official marketplace lists this plugin: its name is not checked against the reserved ones. */
  readonly official?: boolean;
  /** Where the manifest is read: a marketplace entry's gets the shorter advice. Default the plugin's own file. */
  readonly context?: NameContext;
}

export interface NameFindings {
  readonly blocking: readonly Finding[];
  readonly advice: readonly Finding[];
}

/** `reported`: the marketplace entry carries this name and has already said what it thinks of it. */
export const nameFindings = (name: string, options: NameOptions, reported: boolean): NameFindings =>
  call('nameFindings', name, options, reported);
