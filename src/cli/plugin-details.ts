import { coreModule } from '../wasm/bridge.ts';
import type { Rendered } from './result.ts';

const call = coreModule('cli/plugin-details');

/**
 * `claude plugin details <name>` (plugins/cli-reference.md, "plugin details"): the plugin's name,
 * version, description and source, then `Component inventory`, `Projected token cost` and
 * `Per-component (rounded)`; a plugin that is not loaded prints a message and exits `1`. Pure: the
 * host loads the plugin and estimates the tokens. The line layout is the kit's (spec/judgments.md);
 * the section names and the two messages are the docs'.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/plugin_details.rs`, docs/RUST.md).
 */

export interface ComponentCost {
  readonly kind: 'skill' | 'agent' | 'command';
  readonly name: string;
  /** Tokens the component adds to every session. */
  readonly alwaysOn: number;
  /** Tokens it adds when invoked. */
  readonly onInvoke: number;
}

export interface PluginDetails {
  readonly name: string;
  readonly version: string;
  readonly description?: string;
  readonly source: string;
  /** The names of what the plugin contributes, by kind. */
  readonly inventory: {
    readonly skills: readonly string[];
    readonly agents: readonly string[];
    readonly hooks: readonly string[];
    readonly mcpServers: readonly string[];
    readonly lspServers: readonly string[];
  };
  /** The always-on tokens the plugin adds to every session. */
  readonly projectedTokens: number;
  readonly components: readonly ComponentCost[];
}

/** "rounded": to the nearest ten tokens above 100 and the nearest token below. */
export const roundTokens = (tokens: number): number => call('roundTokens', tokens);

/** What the command prints: the details of a loaded plugin, or the message for one that is not loaded. */
export const planPluginDetails = (requested: string, found: PluginDetails | undefined): Rendered =>
  call('planPluginDetails', requested, found);
