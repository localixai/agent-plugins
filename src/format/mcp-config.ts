import type {
  McpHttpServerConfig,
  McpSSEServerConfig,
  McpStdioServerConfig,
} from '../protocol/official.ts';
import type { PluginIssue } from './issues.ts';
import type { UserConfigOption } from './manifest-schema.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * MCP server entries of a plugin (plugins/components.md, "MCP servers"; mcp.md, "Plugin-provided MCP
 * servers"): stdio, SSE, HTTP and WebSocket. The SDK types the first three and not two things the
 * docs give: the `ws` transport and `headersHelper` on the remote ones ("The `type: "ws"` entry
 * accepts the same `url`, `headers`, `headersHelper`, `timeout`, and `alwaysLoad` fields as
 * `http`"). The kit therefore owns `PluginMcpServer`: the official types, plus exactly those fields.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_config.rs`, docs/RUST.md).
 */

const call = coreModule('format/mcp-config');

/** A remote server's `headersHelper`: a command that prints headers (mcp.md, "dynamic headers"). */
interface WithHeadersHelper {
  readonly headersHelper?: string;
}

export type SseMcpServer = McpSSEServerConfig & WithHeadersHelper;
export type HttpMcpServer = McpHttpServerConfig & WithHeadersHelper;
/** `ws` takes the fields of `http` (mcp.md, "Option 4: Add a remote WebSocket server"). */
export type WsMcpServer = Omit<McpHttpServerConfig, 'type'> &
  WithHeadersHelper & { readonly type: 'ws' };
export type RemoteMcpServer = SseMcpServer | HttpMcpServer | WsMcpServer;
export type PluginMcpServer = McpStdioServerConfig | RemoteMcpServer;

/** The transport names of a server's `type`; a server without a `type` is stdio. */
export const MCP_TRANSPORTS: readonly ['stdio', 'sse', 'http', 'ws'] = call('MCP_TRANSPORTS');
export type McpTransport = (typeof MCP_TRANSPORTS)[number];

/** The transport of a server; one without a `type` is stdio. */
export const mcpTransportOf = (server: PluginMcpServer): McpTransport =>
  call('mcpTransportOf', server);

/** "OAuth authentication works with HTTP servers", and the flags apply "to HTTP and SSE transports"; WebSocket "supports neither". */
export const signsInWithOAuth = (server: PluginMcpServer): boolean =>
  call('signsInWithOAuth', server);

/**
 * True for a server whose transport is `sse`, `http` or `ws`. The transport decides, not the
 * presence of a `url`: a stdio entry may carry a `url` key (its `type` is then `stdio`) and stays stdio.
 */
export const isRemoteMcpServer = (server: PluginMcpServer): server is RemoteMcpServer =>
  call('isRemoteMcpServer', server);

/**
 * "Claude Code drops that server and records `Invalid MCP server config for <server> in <path>` only in
 * that debug log" (plugins/troubleshooting.md, "Server is configured but never connects"). The kit
 * reports the same entry as an error issue, which `claude plugin validate` reports too and a host
 * may show or log; this is the line Claude Code writes to its debug log (no quotes around the name).
 */
export const droppedServerLogLine = (server: string, path: string): string =>
  call('droppedServerLogLine', server, path);

export interface McpServerRead {
  /** Undefined when Claude Code would drop the entry. */
  readonly server: PluginMcpServer | undefined;
  readonly issues: readonly PluginIssue[];
}

/**
 * Reads one entry. An entry Claude Code would drop is an error naming the server, in the form
 * plugins/troubleshooting.md quotes: `Invalid MCP server config for "<server>": <error>`. The text
 * after the colon is the kit's (the docs give no text for a malformed entry).
 */
export const readMcpServer = (name: string, raw: unknown, origin: string): McpServerRead =>
  call('readMcpServer', name, raw, origin);

/**
 * Whether an entry of the manifest's inline `mcpServers` keeps the plugin from loading (oracle
 * 2.1.319): the manifest is validated, so any entry that cannot be read fails it, `type: "sdk"`
 * excepted. The same entry in a `.mcp.json` is only dropped.
 */
export const rejectsManifest = (raw: unknown): boolean => call('rejectsManifest', raw);

/**
 * A bundled server's own settings (plugins/components.md, "Include a packaged MCPB server"): what its
 * bundle declares, in the plugin option shape, for a host's settings form and for the resolve. No
 * saved value is kept here: the values are the session's (`pluginConfigs.<id>.mcpServers.<server>`,
 * the host's store under `<server>.<key>`), read when the server is resolved (judgment,
 * spec/judgments.md).
 */
export interface McpBundleConfig {
  readonly userConfig: Readonly<Record<string, UserConfigOption>>;
}

export interface McpConfigRead {
  readonly servers: Record<string, PluginMcpServer>;
  /** The bundled servers among `servers`, by name, with their own settings. */
  readonly bundles?: Record<string, McpBundleConfig>;
  readonly issues: PluginIssue[];
  /** The plugin does not load: an inline manifest entry that cannot be read. */
  readonly rejectsPlugin?: boolean;
}

/**
 * The servers of a `.mcp.json` value (plugins/components.md: "You can also omit the `mcpServers`
 * wrapper and put `db` at the top level of the file"). Judgment: the wrapper is the key `mcpServers`
 * holding an object that is not itself a server (no `command`, no `url`), so a server a plugin
 * names `mcpServers` still reads as a bare map.
 */
export const readMcpConfig = (raw: unknown, origin: string): McpConfigRead =>
  call('readMcpConfig', raw, origin);
