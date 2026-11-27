/**
 * Executables (plugins/manifest-reference.md, "Standard layout"; plugins/components.md,
 * "Executables"). Putting a directory on the Bash tool's `PATH` is the host's effect; what the kit
 * owns is which directory, where it goes among the user's own entries, and which hosts take a plugin
 * that has one.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/bin.rs`, docs/RUST.md).
 */
import type { Locations } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { Plugin } from './plugin-model.ts';

const call = coreModule('loading/bin');

/** The absolute `bin/` directory of a plugin, only when it has one ("while the plugin is enabled"). */
export const binPathEntries = (
  plugin: Pick<Plugin, 'hasBin'>,
  pluginId: string,
  locations: Locations,
): string[] => call('binPathEntries', plugin, pluginId, locations);

/**
 * The `PATH` entries with plugin `bin/` directories added: "Plugin `bin/` directories come after the
 * user's own `PATH` entries, so a plugin can't shadow `git`, `ls`, or another system command". A
 * directory the user already lists keeps its own position.
 */
export const pathWithPluginBins = (
  userEntries: readonly string[],
  binDirectories: readonly string[],
): string[] => call('pathWithPluginBins', userEntries, binDirectories);

/** Where a plugin can be installed: Claude Code and the two hosted surfaces the docs name. */
export const DISTRIBUTION_TARGETS: readonly ['claude-code', 'claude-ai', 'cowork'] =
  call('DISTRIBUTION_TARGETS');
export type DistributionTarget = (typeof DISTRIBUTION_TARGETS)[number];

export type Distribution =
  | { readonly installable: true }
  | { readonly installable: false; readonly reason: string };

/**
 * Whether `target` installs the plugin: "claude.ai and Cowork don't install a plugin that has this
 * directory, including one you distribute through claude.ai organization settings". The target
 * names are the kit's wording of that sentence (judgment).
 */
export const distribution = (
  plugin: Pick<Plugin, 'hasBin'>,
  target: DistributionTarget,
): Distribution => call('distribution', plugin, target);
