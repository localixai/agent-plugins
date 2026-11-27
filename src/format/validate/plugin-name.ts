import type { NameContext } from '../names.ts';

/**
 * The checks of `claude plugin validate` on a manifest's `name` (oracle 2.1.319): an empty name, a
 * space, a control or bidirectional-formatting character are errors and the only report; any
 * other name gets the reserved-name and kebab-case advice, which comes after the schema passes.
 *
 * Types only: the core runs it (`crates/agent-plugins/src/format/validate/plugin_name.rs`,
 * docs/RUST.md), and only the core calls it now.
 */

export interface NameOptions {
  /** An official marketplace lists this plugin: its name is not checked against the reserved ones. */
  readonly official?: boolean;
  /** Where the manifest is read: a marketplace entry's gets the shorter advice. Default the plugin's own file. */
  readonly context?: NameContext;
}
