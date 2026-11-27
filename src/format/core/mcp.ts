import type { PluginIssue } from '../issues.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { CoreMcpServer, CoreTransport } from './mcp-server.ts';
import type { CoreVersion } from './versions.ts';

/**
 * A core package's `mcp.json` (core §7.2): the outer object disables MCP for the package when it
 * fails (core §7.2.2 item 2); a server entry that fails is skipped and reported, the others load
 * (item 3); a valid entry whose transport the host does not support is skipped with a warning, since
 * missing support is not an error (item 4, core §11.3 item 4).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/mcp.rs`, docs/RUST.md).
 */

const call = coreModule('format/core/mcp');

export interface CoreMcpOptions {
  /** The extension namespaces the host implements (core §8.1). */
  readonly implemented?: readonly string[];
  /** The transports the host supports; all three by default (core §7.2.1, "Transport support"). */
  readonly transports?: readonly CoreTransport[];
}

export interface CoreMcpResult {
  readonly servers: Readonly<Record<string, CoreMcpServer>>;
  /** MCP is disabled for the package: the outer object failed. */
  readonly disabled: boolean;
  readonly issues: readonly PluginIssue[];
}

/** Reads a parsed `mcp.json` against the version `plugin.json` targets. */
export const readCoreMcp = (
  raw: unknown,
  manifestVersion: CoreVersion,
  options: CoreMcpOptions = {},
): CoreMcpResult => call('readCoreMcp', raw, manifestVersion, options);
