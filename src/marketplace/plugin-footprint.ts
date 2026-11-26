import { coreModule } from '../wasm/bridge.ts';

/**
 * What an enabled plugin adds to every session (plugins/overview.md, "What an enabled plugin adds to
 * your sessions"): pure over a plain summary of the plugin's components, so no layer above is needed.
 * The token counts are the host's; the kit says which components cost context, which run as processes,
 * and where to look and what to run at each stage.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/plugin_footprint.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/plugin-footprint');

export interface ComponentSummary {
  readonly name: string;
  readonly description?: string;
  /** "Claude can invoke on its own": false for a skill or command with `disable-model-invocation`. */
  readonly modelInvocable: boolean;
}

export interface PluginSummary {
  readonly id: string;
  readonly skills: readonly ComponentSummary[];
  readonly agents: readonly ComponentSummary[];
  readonly commands: readonly ComponentSummary[];
  readonly mcpServers: readonly string[];
  /** The hook events the plugin registers handlers for. */
  readonly hookEvents: readonly string[];
}

export interface ContextEntry {
  readonly kind: 'skill' | 'agent' | 'command';
  readonly name: string;
  readonly description: string;
}

export interface ComponentSummary {
  readonly name: string;
  readonly description?: string;
  /** "Claude can invoke on its own": false for a skill or command with `disable-model-invocation`. */
  readonly modelInvocable: boolean;
}

export interface PluginSummary {
  readonly id: string;
  readonly skills: readonly ComponentSummary[];
  readonly agents: readonly ComponentSummary[];
  readonly commands: readonly ComponentSummary[];
  readonly mcpServers: readonly string[];
  /** The hook events the plugin registers handlers for. */
  readonly hookEvents: readonly string[];
}

export interface ContextEntry {
  readonly kind: 'skill' | 'agent' | 'command';
  readonly name: string;
  readonly description: string;
}

/**
 * "Context and usage": "for each skill, agent, and command that Claude can invoke on its own, the name and
 * description are in Claude's context on every turn", in every session, even when nothing from the plugin
 * runs. "The full text of a skill or agent loads only when it's used", so the body is not an entry.
 */
export const contextEntries = (plugin: PluginSummary): readonly ContextEntry[] =>
  call('contextEntries', plugin);

export interface Processes {
  /** "MCP servers the plugin defines run alongside each session where it's enabled". */
  readonly mcpServers: readonly string[];
  /** "its hooks fire at their events". */
  readonly hookEvents: readonly string[];
}

export const processesOf = (plugin: PluginSummary): Processes => call('processesOf', plugin);

/** "What the plugin runs, it runs as you." */
export const runsAs: 'the user' = call('runsAs');

/** "Plugins in Anthropic's official marketplace show a **Context cost** estimate" in the Marketplaces tab. */
export const showsContextCost = (marketplace: string): boolean =>
  call('showsContextCost', marketplace);

/** The three stages the docs name and where to look or what to run at each. */
export interface FootprintGuide {
  readonly beforeInstall: { readonly where: string; readonly contextCost: boolean };
  readonly afterInstall: { readonly where: readonly string[] };
  readonly stopWithoutUninstalling: { readonly panel: string; readonly shell: string };
}

export const footprintGuide = (id: string, marketplace: string): FootprintGuide =>
  call('footprintGuide', id, marketplace);
