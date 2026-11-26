import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/plugin-ref');

/**
 * The `<plugin>` argument of the plugin commands (plugins/cli-reference.md, "claude plugin
 * commands": "a `<plugin>` argument is a plugin `name` or `name@marketplace`. When two marketplaces
 * offer the same name, use the qualified form"; "plugin update", "Update by bare name": a bare name
 * "matches against your installed plugins"; when the same name is installed from different
 * marketplaces "the command refuses ... and lists the qualified `plugin-name@marketplace-name`
 * commands to run instead"). Pure over the ids the host lists.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/plugin_ref.rs`, docs/RUST.md).
 */

/** The name part of an id (`formatter` of `formatter@my-marketplace`). */
export const nameOf = (id: string): string => call('nameOf', id);

/**
 * The ids of `ids` the argument names: a qualified reference matches one id exactly, a bare name
 * every id with that name. No hit is "not installed"; two or more are ambiguous.
 */
export const matchingIds = (text: string, ids: readonly string[]): readonly string[] =>
  call('matchingIds', text, ids);

/** The refusal of an ambiguous bare name: the qualified commands to run instead. */
export const qualifiedCommands = (verb: string, ids: readonly string[]): readonly string[] =>
  call('qualifiedCommands', verb, ids);
