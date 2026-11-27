import type { PluginIssue } from '../../format/issues.ts';
import type { PluginMcpServer } from '../../format/mcp-config.ts';
import type { PolicyLayer } from '../layers.ts';
import type { ManagedServerOptions } from './managed.ts';
import type { McpCandidate } from './scope.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The servers the organization provides through `managedMcpServers`, and what `/mcp`, `claude mcp
 * get` and `claude mcp remove` show of one.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/managed_servers.rs`, docs/RUST.md).
 */
const call = coreModule('settings/mcp/managed-servers');

export interface ProvidedServers {
  /** The servers the organization provides, as managed-scope candidates, in the order the sources define them. */
  readonly servers: readonly McpCandidate[];
  /** Why an entry was not loaded: the notices `/status` shows. */
  readonly issues: readonly PluginIssue[];
}

/**
 * The servers the organization provides through `managedMcpServers` (managed-mcp.md, "Provide servers
 * through managed settings"; v2.1.259): the managed layers' entries (`managedServerEntries`: managed
 * sources only, names across sources union, the higher source's entry whole), each kept only when it
 * passes every check of "What an entry can contain"; one that fails is dropped with a notice and the
 * others still load. Hand the result to `mcpLoadPlan`; it ranks above every other scope.
 */
export const readManagedMcpServers = (
  layers: readonly PolicyLayer[],
  options: ManagedServerOptions = {},
): ProvidedServers => call('readManagedMcpServers', layers, options);

/** `/mcp` "lists provided servers under **Managed MCPs**". */
export const MANAGED_MCP_GROUP: 'Managed MCPs' = call('MANAGED_MCP_GROUP');

export interface ManagedServerDescription {
  readonly group: typeof MANAGED_MCP_GROUP;
  /** "the URL's host only, for example `https://mcp.example.com/…`". */
  readonly url: string;
  /** "its header names without their values". */
  readonly headerNames: readonly string[];
}

/** What `claude mcp get` and `/mcp` show of a provided server (managed-mcp.md, "What users can see and change"). */
export const describeManagedServer = (server: PluginMcpServer): ManagedServerDescription =>
  call('describeManagedServer', server);

/** What `claude mcp remove` reports for a provided server (managed-mcp.md, "How restrictions appear to users"). */
export const managedServerRemoval = (name: string): string => call('managedServerRemoval', name);
