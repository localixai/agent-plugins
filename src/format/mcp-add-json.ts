import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { PluginMcpServer } from './mcp-config.ts';
import type { EnvLookup } from './mcp-env.ts';
import type { McpOAuthConfig } from './mcp-oauth.ts';

/**
 * `claude mcp add-json <name> '<json>'` and the part of `claude mcp add` that both share (mcp.md, "Add
 * MCP servers from JSON configuration", "Use pre-configured OAuth credentials", "Add a server from
 * setup instructions written for another client"): the input becomes a server entry, the OAuth
 * credentials the add carries, and what the host must ask for. The stored configuration, the keychain
 * and the prompt are the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_add_json.rs`, docs/RUST.md).
 */

const call = coreModule('format/mcp-add-json');

export const MCP_ADD_SCOPES: readonly ['local', 'project', 'user'] = call('MCP_ADD_SCOPES');
export type McpAddScope = (typeof MCP_ADD_SCOPES)[number];

/** Where the client secret comes from: "The `--client-secret` flag prompts", or `MCP_CLIENT_SECRET`. */
export type McpSecretRequest =
  | { readonly source: 'prompt' }
  | { readonly source: 'environment'; readonly value: string };

export interface McpAddOptions {
  /** `--scope`; local when absent. */
  readonly scope?: string;
  /** `--client-secret`. */
  readonly clientSecret?: boolean;
  /** The servers already stored, as `mcpAddKey(scope, name)`: an add of one of them fails. */
  readonly existing?: ReadonlySet<string>;
}

export interface McpAddRead {
  readonly name: string;
  readonly scope: McpAddScope | undefined;
  /** Undefined when the add is refused. */
  readonly server: PluginMcpServer | undefined;
  /** The OAuth setup that applies: the entry's `oauth` block, with `MCP_OAUTH_CALLBACK_PORT` filled in. */
  readonly oauth: McpOAuthConfig | undefined;
  /** Present only with `--client-secret` on an HTTP or SSE server. Never stored in `server`. */
  readonly secret: McpSecretRequest | undefined;
  readonly issues: readonly PluginIssue[];
}

/** "Claude Code adds no server named like that": the message of `claude mcp add` for a name taken at that scope. */
export const mcpServerExistsMessage = (name: string, scope: McpAddScope): string =>
  call('mcpServerExistsMessage', name, scope);

/**
 * How the host names a stored server for `McpAddOptions.existing`: the scope and the name, which
 * together make "the same server name at the same scope". A name has no `:` (see `NAME`).
 */
export const mcpAddKey = (scope: McpAddScope, name: string): string =>
  call('mcpAddKey', scope, name);

/**
 * `claude mcp add-json <name> '<json>'`: "The JSON must conform to the MCP server configuration
 * schema" (`readMcpServer`). The wording of a refusal other than `Invalid MCP server config for
 * "<server>": <error>` and the two messages mcp.md quotes is the kit's (spec/judgments.md).
 */
export const readMcpAddJson = (
  name: string,
  json: string,
  options: McpAddOptions,
  lookup: EnvLookup,
): McpAddRead => call('readMcpAddJson', name, json, options, lookup);
