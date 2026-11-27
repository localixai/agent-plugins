/**
 * The allowlist and the denylist over one server (managed-mcp.md, "How a server is evaluated").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/allow_deny.rs`, docs/RUST.md).
 */
import type { PluginMcpServer } from '../../format/mcp-config.ts';
import type { PolicyEnv } from '../context.ts';
import type { PolicyView } from '../view.ts';
import type { McpServerOrigin } from './origins.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/allow-deny');

export interface McpPolicyEnv {
  /** The environment Claude Code started with: the base of what a policy entry expands in. */
  readonly startup: PolicyEnv;
  /** The live process environment: what the server's own `${VAR}` expands in. */
  readonly live: PolicyEnv;
}

export interface McpPolicySubject {
  /** The label the server goes by: the key in its file, or a claude.ai connector's display name. */
  readonly name: string;
  readonly server: PluginMcpServer;
  /** Defaults to `configured`: a server a user, a plugin or claude.ai adds. */
  readonly origin?: McpServerOrigin;
}

/** The lists that blocked a server. */
export type McpBlockedBy = 'denylist' | 'allowlist';

export type McpServerVerdict =
  | { readonly allowed: true }
  | {
      readonly allowed: false;
      readonly blockedBy: McpBlockedBy;
      /** The status of the server in `claude mcp list` and `claude mcp get`. */
      readonly status: string;
      /** What `claude mcp add` says, and what a reconnect says (managed-mcp.md, "How restrictions appear to users"). */
      readonly addError: string;
      readonly reconnectError: string;
    };

export interface McpServerPolicy {
  /** Whether the server may load, and why not. */
  check(subject: McpPolicySubject): McpServerVerdict;
}

/**
 * The allowlist and the denylist over one server (managed-mcp.md, "How a server is evaluated"), in
 * the order the page gives: merge the lists (`lists.ts`), "check the denylist" ("Nothing overrides a
 * denylist match"), "check the allowlist" ("If `allowedMcpServers` isn't set anywhere, every server
 * that passed the denylist loads"). A remote server is allowed by a `serverUrl` entry, a stdio server
 * by a `serverCommand` entry, and a `serverName` match counts only when the list holds no entry of the
 * server's kind. The origins that skip a check are `origins.ts`.
 *
 * The lists are read once, when the policy is created, so one policy gives the same verdict for a
 * server until it is dropped: a host that sees settings change before a user reconnects a server (the
 * docs run the checks again then) builds a new policy, as `McpSettingsPolicy.serverPolicy()` does.
 */
export const createMcpServerPolicy = (view: PolicyView, env: McpPolicyEnv): McpServerPolicy =>
  call('createMcpServerPolicy', view, { startup: env.startup, live: env.live });
