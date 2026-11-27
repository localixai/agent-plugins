import type { CommandEntryContext } from '../format/command.ts';
import type { PluginIssue } from '../format/issues.ts';
import type { CommandEntry } from '../format/manifest-schema.ts';
import type { Command } from '../format/skill-types.ts';
import type { PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The manifest's `commands` object map (plugins/manifest-reference.md, "commands"; plugins/
 * components.md, "Define commands in the manifest"): each entry becomes a command named after the
 * plugin prefix and its key, with its body from inline `content` or from the `source` file, read
 * through the `PluginFiles` port. An entry that cannot load is an error issue and the others still
 * load: a plugin error is a value, never a throw.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/command_entries.rs`, docs/RUST.md).
 */

const call = coreModule('skills/command-entries');

export interface CommandEntriesLoaded {
  /** In the map's key order. */
  readonly commands: readonly Command[];
  readonly issues: readonly PluginIssue[];
}

export interface Grants {
  readonly rules: readonly string[];
  readonly issues: readonly PluginIssue[];
}

/**
 * The rules of `allowedTools` that parse as permission rules (permissions.md, "Permission rule
 * syntax"), as written. Judgment (spec/judgments.md): a rule that does not parse is dropped with a
 * warning and grants nothing, and the command still loads; a grant is read like a session rule.
 */
export const commandEntryGrants = (key: string, rules: readonly string[]): Grants =>
  call('commandEntryGrants', key, rules);

/**
 * Load every entry of a `commands` object map. `files` is the plugin folder; a `source` is read
 * from it only when the entry sets `source` alone.
 */
export const loadCommandEntries = (
  map: Readonly<Record<string, CommandEntry>>,
  files: PluginFiles,
  ctx: CommandEntryContext,
  options: { readonly signal: AbortSignal },
): Promise<CommandEntriesLoaded> => call.async('loadCommandEntries', map, files, ctx, options);
