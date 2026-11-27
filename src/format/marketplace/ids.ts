/**
 * Each part of a plugin id, `plugin@marketplace` (plugins/marketplace-reference.md, "Validation
 * messages"; plugins/troubleshooting.md, "Cannot add marketplace"): letters a-z and A-Z, digits,
 * `.`, `_` and `-`, starting with a letter or digit. Shared by entries, rename targets and
 * dependencies, so the rule exists once.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/ids.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';

const call = coreModule('format/marketplace/ids');

export const PLUGIN_ID_PART: RegExp = call('PLUGIN_ID_PART');

export const isPluginIdPart = (name: string): boolean => call('isPluginIdPart', name);

export type IdPart = 'marketplace' | 'plugin';

/**
 * What a marketplace or entry `name` breaks before it can be part of an id: a space, a control or
 * bidirectional-formatting character, for a marketplace a path separator or `..`, the grammar. Each rule that fires is reported (the docs list
 * them as separate rows), on `path`. An empty name is left to the reader's own "must have a name".
 */
export const pluginIdIssues = (part: IdPart, name: string, path: string): PluginIssue[] =>
  call('pluginIdIssues', part, name, path);

/** Warnings for names Claude Desktop rejects: the reserved marketplace names and the 128-character grammar. */
export const desktopNameIssues = (part: IdPart, name: string, path: string): PluginIssue[] =>
  call('desktopNameIssues', part, name, path);

/** A `<plugin>` reference: `name`, or `name@marketplace` (plugins/cli-reference.md, "claude plugin commands"). */
export interface PluginRef {
  readonly name: string;
  /** Absent for a bare name. */
  readonly marketplace?: string;
}

/** `name` or `name@marketplace`; any other number of `@`, or an empty part, is no reference. */
export const parsePluginRef = (text: string): PluginRef | undefined => call('parsePluginRef', text);

/** The name part of a plugin id (`formatter` of `formatter@my-marketplace`); a bare name is its own. */
export const pluginNameOf = (id: string): string => call('pluginNameOf', id);
