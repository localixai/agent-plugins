/**
 * MCP bundles (plugins/manifest-reference.md, "mcpServers"; plugins/components.md, "Include a
 * packaged MCPB server"). The host opens a bundle (extracts or downloads it into `.mcpb-cache/`
 * under the plugin root, reads its manifest into a server config); the kit decides what a manifest
 * value is, that it ends in `.mcpb` or `.dxt`, which server name results and what the bundle's
 * settings are. Whether the server may start and what its `${user_config.KEY}` references stand for
 * are decided when it is resolved, from the session's saved values (`loading/resolve/bundle-scope.ts`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_bundles.rs`, docs/RUST.md).
 */
import type { McpBundleReader, McpBundleSource } from '../ports/mcp-bundle.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { LocateContext } from './locate.ts';

const call = coreModule('format/mcp-bundles');

/**
 * What a source needs to load: the plugin's files and what decides whether a path leaves them
 * (`locate`), and, optionally, the host's capabilities.
 */
export interface McpLoadContext extends LocateContext {
  /** Absent: a bundle is a warning, never a crash. */
  readonly bundles?: McpBundleReader;
}

/** One `--config` flag of `claude plugin install`: a plugin option, or a setting of a bundled server. */
export interface ConfigFlag {
  /** The bundled server, for a key written `<server>.<key>`; absent for a plugin `userConfig` option. */
  readonly server?: string;
  readonly key: string;
  readonly value: string;
}

/**
 * `--config <key=value>` (plugins/cli-reference.md, "plugin install"): "A key written `<server>.<key>`
 * sets a setting that a bundled MCP server declares in its own `user_config` instead". Judgment
 * (spec/judgments.md): the first `=` ends the key and the first `.` of the key ends the server name,
 * so a value keeps its own `=` and a setting key may hold dots; a flag with no `=` or an empty part is
 * an error.
 */
export const readConfigFlag = (text: string): ConfigFlag | PluginIssue =>
  call('readConfigFlag', text);

/**
 * "For a bundle file shipped inside the plugin": a `<server>.<key>` flag works only when the bundle is
 * a path in the plugin, never for one downloaded from a URL.
 */
export const configFlagApplies = (source: McpBundleSource): boolean =>
  call('configFlagApplies', source);
