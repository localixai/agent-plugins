/**
 * Which servers load under the organization's MCP configuration, and what Claude Code says and does
 * about the ones that do not (managed-mcp.md, "Exclusive control with managed-mcp.json").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/exclusive.rs`, docs/RUST.md).
 */
import type { PluginMcpServer } from '../../format/mcp-config.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { McpServerPolicy, McpServerVerdict } from './allow-deny.ts';
import type { McpLoadSettings } from './load-settings.ts';
import type { ManagedMcpFile } from './managed-file.ts';
import type { McpServerOrigin } from './origins.ts';
import type { HiddenMcpServer, McpCandidate, McpScope } from './scope.ts';
import type { ProjectToggles } from './toggles.ts';

const call = coreModule('settings/mcp/exclusive');

/** `claude mcp add` while `managed-mcp.json` is deployed (managed-mcp.md, "Validate the configuration"). */
export const EXCLUSIVE_ADD_ERROR: string = call('EXCLUSIVE_ADD_ERROR');

/** The startup exit of `--mcp-config` on a workstation (managed-mcp.md, "Servers passed with --mcp-config"). */
export const MCP_CONFIG_EXIT: string = call('MCP_CONFIG_EXIT');

/**
 * The startup exit of `--strict-mcp-config`. The page says Claude Code "exits at startup" without
 * quoting it (spec/judgments.md).
 */
export const STRICT_MCP_CONFIG_EXIT: string = call('STRICT_MCP_CONFIG_EXIT');

/** The startup exit of `claude --chrome` (managed-mcp.md, "How restrictions appear to users"). */
export const CHROME_BLOCKED_ERROR: string = call('CHROME_BLOCKED_ERROR');

export interface NamedMcpServer {
  readonly name: string;
  readonly server: PluginMcpServer;
}

export interface McpSessionFacts {
  /** A cloud session: the host delivers connectors and other servers through `--mcp-config`. */
  readonly cloud: boolean;
  /** The servers `--mcp-config` passes (in a cloud session, the ones the cloud host delivers). */
  readonly mcpConfig?: readonly NamedMcpServer[];
  readonly strictMcpConfig?: boolean;
  /** Set only for a user who could run Claude in Chrome: they ran `claude --chrome`, or turned it on by default. */
  readonly chrome?: { readonly flag: boolean; readonly byDefault: boolean };
}

/** A server some scope other than the organization's defines; the plan builds the managed ones itself. */
export type ConfiguredCandidate = McpCandidate & { readonly scope: Exclude<McpScope, 'managed'> };

export interface McpLoadInput {
  /** `readManagedMcpFile(...).file`: undefined when no file is deployed or it cannot be read. */
  readonly managedFile: ManagedMcpFile | undefined;
  /** `readManagedMcpServers(...).servers`. */
  readonly managedServers: readonly McpCandidate[];
  readonly candidates: readonly ConfiguredCandidate[];
  readonly settings: McpLoadSettings;
  readonly policy: McpServerPolicy;
  readonly session: McpSessionFacts;
  /** The project's `/mcp` toggles; a disabled server stays in the plan, marked. */
  readonly toggles?: ProjectToggles | undefined;
}

/** Where a planned server comes from; the two organization sources are told apart for the lists. */
export const PLAN_SOURCES: readonly [
  'managedFile',
  'managedSettings',
  'local',
  'project',
  'user',
  'plugin',
  'connector',
  'flag',
] = call('PLAN_SOURCES');
export type PlanSource = (typeof PLAN_SOURCES)[number];

export interface PlannedServer extends NamedMcpServer {
  readonly source: PlanSource;
  readonly origin: McpServerOrigin;
  /** `false` when the user turned it off in `/mcp`. */
  readonly enabled: boolean;
}

export type BlockedServer = NamedMcpServer & {
  readonly source: PlanSource;
} & Extract<McpServerVerdict, { readonly allowed: false }>;

export interface McpLoadPlan {
  /** Why Claude Code exits at startup, or `undefined` when the session starts. */
  readonly exit: string | undefined;
  /** A readable `managed-mcp.json` is deployed. */
  readonly exclusive: boolean;
  /** What loads, highest scope first (servers of `--mcp-config` last). */
  readonly servers: readonly PlannedServer[];
  /** What `allowedMcpServers` and `deniedMcpServers` stop. */
  readonly blocked: readonly BlockedServer[];
  /** What a server above displaced. */
  readonly hidden: readonly HiddenMcpServer[];
  /** The servers a cloud host delivered that exclusive control skipped. */
  readonly skipped: readonly string[];
  /** What Claude Code writes to stderr. */
  readonly warnings: readonly string[];
  /** What `claude mcp add` says while the file is deployed. */
  readonly addError: string | undefined;
  /** The built-in Claude in Chrome server runs. */
  readonly chrome: boolean;
}

/**
 * Which servers load, and what Claude Code says and does about the ones that do not (managed-mcp.md,
 * "Exclusive control with managed-mcp.json" and the sections under it). With a `managed-mcp.json` that
 * can be read and parsed, only its servers and the provided ones load; the lists (`deniedMcpServers`
 * always, `allowedMcpServers` for what is not the organization's own) filter the result. Without it
 * the scopes rank as usual and the flags keep their meaning: a `--mcp-config` server replaces a
 * provided one of its name and meets the allowlist, `--strict-mcp-config` leaves everything else out.
 * The built-in in-process servers of the app that started the session never pass through here.
 */
export const mcpLoadPlan = (input: McpLoadInput): McpLoadPlan => call('mcpLoadPlan', input);
