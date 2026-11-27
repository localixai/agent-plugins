import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';
import type { PluginMcpServer } from './mcp-config.ts';

/**
 * The `oauth` block of a remote server's entry and what it decides (mcp.md, "Use a fixed OAuth
 * callback port", "Use pre-configured OAuth credentials", "Override OAuth metadata discovery",
 * "Restrict OAuth scopes", "Use OAuth with a server behind an authenticating proxy", "Authenticate
 * with remote MCP servers"). Pure data: the OAuth flow itself, the browser, the token store and the
 * HTTP requests are the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_oauth.rs`, docs/RUST.md).
 */
export interface McpOAuthConfig {
  /** The pre-registered client's ID. */
  readonly clientId?: string;
  /** The fixed port of the redirect URI `http://localhost:PORT/callback`. */
  readonly callbackPort?: number;
  /** "The URL must use `https://`": the metadata document that replaces the discovery chain. */
  readonly authServerMetadataUrl?: string;
  /** "A single space-separated string, matching the `scope` parameter format in RFC 6749 §3.3". */
  readonly scopes?: string;
}

export interface McpOAuthRead {
  /** Undefined when the block is absent or one of its fields is refused. */
  readonly oauth: McpOAuthConfig | undefined;
  readonly issues: readonly PluginIssue[];
}

const call = coreModule('format/mcp-oauth');

/**
 * Reads the value of a server's `oauth` key. Judgments (spec/judgments.md): a field the docs do not
 * list is a warning and is not kept (so a secret never travels in the entry); a field of the wrong
 * type makes the whole block unusable, since half an OAuth setup would sign in with another client.
 */
export const readMcpOAuth = (name: string, raw: unknown, origin: string): McpOAuthRead =>
  call('readMcpOAuth', name, raw, origin);

/** The redirect URI a fixed callback port registers (mcp.md: `http://localhost:PORT/callback`; never `127.0.0.1`). */
export const mcpOAuthRedirectUri = (port: number): string => call('mcpOAuthRedirectUri', port);

/** Where the authorization server's metadata is looked for. */
export type McpOAuthDiscovery =
  | { readonly kind: 'configured'; readonly url: string }
  | { readonly kind: 'chain'; readonly urls: readonly string[] };

/**
 * "By default, Claude Code first checks RFC 9728 Protected Resource Metadata at
 * `/.well-known/oauth-protected-resource`, then falls back to RFC 8414 authorization server metadata
 * at `/.well-known/oauth-authorization-server`"; `authServerMetadataUrl` bypasses that chain.
 * Judgment (spec/judgments.md): both paths are taken from the server's origin, as the docs write them
 * (RFC 9728 and RFC 8414 also define forms that insert the path of the resource).
 */
export const mcpOAuthDiscovery = (
  serverUrl: string,
  config: McpOAuthConfig | undefined,
): McpOAuthDiscovery => call('mcpOAuthDiscovery', serverUrl, config);

/** The metadata of an authorization server as far as scopes go. */
export interface McpOAuthMetadata {
  /** `configured`: fetched from `authServerMetadataUrl`; `discovered`: found by the chain. */
  readonly source: 'configured' | 'discovered';
  readonly scopesSupported?: readonly string[];
}

/**
 * The `scope` of the authorization request, or `undefined` for none (the server decides). "`oauth.scopes`
 * takes precedence over both `authServerMetadataUrl` and the scopes the server discovers"; when the
 * server advertises `offline_access` it is appended to the pinned scopes. Unset: discovered metadata
 * does not put its whole `scopes_supported` catalog in the request, while metadata fetched from a
 * configured `authServerMetadataUrl` "still supplies its `scopes_supported` as the requested scopes".
 */
export const requestedOAuthScopes = (
  config: McpOAuthConfig | undefined,
  metadata: McpOAuthMetadata,
): string | undefined => call('requestedOAuthScopes', config, metadata);

export type McpOAuthPlan =
  | {
      readonly oauth: true;
      /** Sent with the OAuth requests too, so a proxy that checks a header of its own lets them through. */
      readonly headers: Readonly<Record<string, string>>;
      readonly config?: McpOAuthConfig;
    }
  | { readonly oauth: false; readonly reason: 'transport' | 'authorization-header' };

/**
 * Whether a server signs in with OAuth, and with which headers. "OAuth authentication works with HTTP
 * servers" and the flags apply "to HTTP and SSE transports"; WebSocket "supports neither" and a stdio
 * server has no URL. "If you configure an `Authorization` header, Claude Code uses it as the server's
 * credential and doesn't fall back to OAuth for that server": `headers` are the ones the connection
 * sends (static headers with a `headersHelper`'s output merged in, as `connectionHeaders` does).
 */
export const planMcpOAuth = (
  server: PluginMcpServer,
  connection: {
    readonly headers: Readonly<Record<string, string>>;
    readonly oauth?: McpOAuthConfig;
  },
): McpOAuthPlan => call('planMcpOAuth', server, connection);

/** What a rejected request means for a server (mcp.md, "Authenticate with remote MCP servers"). */
export interface McpAuthRejectionInput {
  readonly status: number;
  /** A configured server, a claude.ai connector, or a connector delivered to a cloud session. */
  readonly source: 'server' | 'connector' | 'cloud-connector';
  /** An `Authorization` header configured in `headers` or produced by a `headersHelper`. */
  readonly authorizationConfigured: boolean;
  /** For a connector: the 401 is claude.ai rejecting the session token. */
  readonly sessionTokenRejected?: boolean;
}

export type McpAuthState =
  | 'needs-authentication'
  | 'failed'
  | 'session-token-rejected'
  | 'reconnect-in-claude-ai'
  | 'not-a-rejection';

export const mcpAuthRejection = (input: McpAuthRejectionInput): McpAuthState =>
  call('mcpAuthRejection', input);

export interface StoredTokenRecovery {
  readonly first: 'refresh-and-retry' | 'connect-without-token';
  /** When `/mcp` flags the server: the retry failed, or the connection without the token was rejected too. */
  readonly flaggedWhen: 'retry-fails' | 'connection-rejected';
  /** "Its tools work while you're signed out, and the rejected token is deleted." */
  readonly deletesRejectedToken: boolean;
}

/** A stored OAuth token rejected with 401: "Your stored login has a refresh token" or not. */
export const storedTokenRecovery = (login: {
  readonly refreshToken: boolean;
}): StoredTokenRecovery => call('storedTokenRecovery', login);
