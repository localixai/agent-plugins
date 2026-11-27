/** Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/managed.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { PolicyLayer } from '../layers.ts';

const call = coreModule('settings/mcp/managed');

export interface ManagedServerEntries {
  /** Entries by server name, as written, for the entry validation to read. */
  readonly entries: Readonly<Record<string, unknown>>;
  readonly issues: readonly PluginIssue[];
}

/**
 * Sessions of the Claude Desktop app where Claude Code does not read the key: "the Claude Desktop
 * app's Code tab on a third-party deployment or ... the app's Cowork sessions, because Claude Desktop
 * supplies and locks those sessions' MCP servers itself".
 */
export type DesktopManaged = 'third-party' | 'cowork';

export interface ManagedServerOptions {
  /** The session is one of these Desktop sessions: no managed entry is read and a managed layer that carries the key is reported. */
  readonly desktopManaged?: DesktopManaged;
}

/**
 * The servers the organization provides: `managedMcpServers` of the managed layers only. The layers
 * are the managed sources the host selected, highest rank first; "when two sources define the same
 * name, the higher-ranked source's entry applies whole". Any other layer that carries the key has it
 * dropped with a warning. In a Desktop session that supplies its own servers (`desktopManaged`) no
 * entry is read; `/status` and `claude doctor` "say so when your managed settings carry the key
 * there", which this reports as a warning for each managed layer that carries it.
 */
export const managedServerEntries = (
  layers: readonly PolicyLayer[],
  options: ManagedServerOptions = {},
): ManagedServerEntries => call('managedServerEntries', layers, options);
