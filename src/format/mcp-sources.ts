import type { PluginFiles } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { LocateOptions } from './locate.ts';
import type { McpLoadContext } from './mcp-bundles.ts';
import type { McpBundleConfig, McpConfigRead, PluginMcpServer } from './mcp-config.ts';

/**
 * Where a plugin's MCP servers come from (plugins/manifest-reference.md, "mcpServers"): "Claude Code
 * loads `.mcp.json` at the plugin root first, then each declared shape in order. A server name
 * declared later replaces an earlier one." The four shapes of the table are the `McpSourceKind`
 * members; each has one strategy in a registry typed over the union.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_sources.rs`, docs/RUST.md).
 */

const call = coreModule('format/mcp-sources');

export const MCP_SOURCE_KINDS: readonly ['json-file', 'bundle-path', 'bundle-url', 'inline-map'] =
  call('MCP_SOURCE_KINDS');
export type McpSourceKind = (typeof MCP_SOURCE_KINDS)[number];

/** One declared source, ready to load. */
export interface McpSource {
  readonly kind: McpSourceKind;
  load(context: McpLoadContext): Promise<McpConfigRead>;
}

/** What a manifest `mcpServers` value (or one element of an array) is, or `undefined`. */
export const classifyMcpSource = (value: unknown): McpSource | undefined =>
  call('classifyMcpSource', value);

/**
 * What decides containment (`platform`, `roots`), and the host's capabilities: a bundle opener
 * (without it a bundle is a warning) and saved bundle settings.
 */
export type CollectOptions = LocateOptions & Omit<McpLoadContext, keyof LocateOptions | 'files'>;

export interface CollectedMcpServers {
  readonly servers: Record<string, PluginMcpServer>;
  /** The bundled servers among `servers`, by name: a later source's server of the same name drops it. */
  readonly bundles: Record<string, McpBundleConfig>;
  readonly issues: PluginIssue[];
  /** An inline manifest entry could not be read: the plugin does not load (oracle 2.1.319). */
  readonly rejectsPlugin?: boolean;
}

/**
 * Every server of a plugin: the root `.mcp.json` first, then each declared source in array order;
 * a later name replaces an earlier one (a manifest server replaces a `.mcp.json` one,
 * components.md). A source that fails leaves the others loading.
 */
export const collectMcpServers = (
  files: PluginFiles,
  manifestValue: unknown,
  options: CollectOptions = {},
): Promise<CollectedMcpServers> => call.async('collectMcpServers', files, manifestValue, options);
