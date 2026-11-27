import { coreModule } from '../wasm/bridge.ts';
import type { McpTransport } from './mcp-config.ts';
import type { EnvLookup } from './mcp-env.ts';

/**
 * The environment variables that decide how MCP servers connect at startup and how long a call runs
 * before it moves to the background (env-vars.md, "Variables"; mcp.md, "Automatic backgrounding of long
 * tool calls", "Server status detail", "MCP client runtimes"; agent-sdk/mcp.md, "Connection timing").
 * Pure over a lookup of the environment: connecting, the cache on disk and the clock are the host's.
 * A value that is not a number (digits, `2e3`, `64_000`: spec §3.6.1 item 2, J3.17) is ignored.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_startup.rs`, docs/RUST.md).
 */

const call = coreModule('format/mcp-startup');

export interface McpConnectionBatches {
  /** `MCP_SERVER_CONNECTION_BATCH_SIZE`: stdio servers connecting at once. */
  readonly local: number;
  /** `MCP_REMOTE_SERVER_CONNECTION_BATCH_SIZE`: HTTP and SSE servers connecting at once. */
  readonly remote: number;
}

export const mcpConnectionBatches = (lookup: EnvLookup): McpConnectionBatches =>
  call('mcpConnectionBatches', lookup);

export interface McpBackgrounding {
  /** Milliseconds before a running call moves to a background task; `undefined` when that is off. */
  readonly afterMs: number | undefined;
  /** In an interactive session Claude waits for the result of an MCP task instead of working on. */
  readonly waitsForTasks: boolean;
}

export const mcpBackgrounding = (lookup: EnvLookup): McpBackgrounding =>
  call('mcpBackgrounding', lookup);

export interface McpStartupServer {
  /** `alwaysLoad: true`: "their tools must be present when the first prompt is built". */
  readonly alwaysLoad: boolean;
  /** Its tool list comes from a valid discovery-cache entry: it connects on its first tool call. */
  readonly cached: boolean;
  /** Its transport: only an `http` or `sse` server is served from the discovery cache (spec §8.8 item 1). */
  readonly transport: McpTransport;
}

export interface McpStartupRun {
  /** `claude -p` without `--input-format stream-json`. */
  readonly printMode: boolean;
}

export const mcpStartupWaitsFor = (
  lookup: EnvLookup,
  server: McpStartupServer,
  run: McpStartupRun,
): boolean => call('mcpStartupWaitsFor', lookup, server, run);

export type McpFirstTurnServer = 'permission-prompt' | 'sdk' | 'named' | 'other';

export const mcpFirstTurnWaitMs = (
  lookup: EnvLookup,
  server: McpFirstTurnServer,
): number | undefined => call('mcpFirstTurnWaitMs', lookup, server);

export interface McpDiscoveryCache {
  /** Whether the cache is on for this session. */
  readonly enabled: boolean;
  /** `MCP_DISCOVERY_CACHE_TTL_S`: seconds an entry is used without a refresh. */
  readonly ttlSeconds: number;
  /** `MCP_DISCOVERY_CACHE_MAX_STALE_S`: seconds after which an entry is discarded. */
  readonly maxStaleSeconds: number;
  /** `MCP_DISCOVERY_CACHE_STRIKES`: failed refreshes in a row that discard an entry. */
  readonly strikes: number;
}

export const mcpDiscoveryCache = (
  lookup: EnvLookup,
  session: { readonly rolledOut: boolean },
): McpDiscoveryCache => call('mcpDiscoveryCache', lookup, session);

export type McpCacheUse = 'use' | 'use-and-refresh' | 'discard';

export const mcpCacheEntryUse = (cache: McpDiscoveryCache, ageSeconds: number): McpCacheUse =>
  call('mcpCacheEntryUse', cache, ageSeconds);

export const mcpCacheDiscardedAfterFailures = (
  cache: McpDiscoveryCache,
  failures: number,
): boolean => call('mcpCacheDiscardedAfterFailures', cache, failures);

export type McpClientRuntime = 'v1' | 'v2';

export type McpProtocolNegotiation = 'legacy' | 'auto' | 'default';

export interface McpClientSettings {
  readonly runtime: McpClientRuntime;
  /**
   * `MCP_PROTOCOL_NEGOTIATION`, which applies "on the v2 MCP client runtime only": `legacy` keeps every
   * server on the earlier handshake, `auto` probes the kinds that support revision 2026-07-28, and
   * `default` (unset) probes them but may skip some. `undefined` on v1.
   */
  readonly negotiation: McpProtocolNegotiation | undefined;
  /** The v2 runtime "checks the issuer an MCP OAuth server returns in its authorization response". */
  readonly checksIssuer: boolean;
  /** For the debug log: "any other value is ignored with a warning". The wording is the kit's. */
  readonly warnings: readonly string[];
}

export const mcpClientSettings = (lookup: EnvLookup): McpClientSettings =>
  call('mcpClientSettings', lookup);

export const registersAsChannel = (runtime: McpClientRuntime, protocolRevision: string): boolean =>
  call('registersAsChannel', runtime, protocolRevision);
