/**
 * `--mcp-config` and `--strict-mcp-config` (cli-reference.md, "CLI flags"; plugins/org.md,
 * `disableSideloadFlags`; headless.md, "Start faster with bare mode"). `--mcp-config` loads "MCP
 * servers from JSON files or strings"; with `-p` Claude Code "waits for still-pending servers to
 * connect before running the first turn, up to the `MCP_TIMEOUT` startup timeout, 30 seconds by
 * default". `--strict-mcp-config` means "only use MCP servers from `--mcp-config`, ignoring all other
 * MCP configurations". `disableSideloadFlags` rejects `--mcp-config` "unless every server in it is an
 * in-process `type: "sdk"` entry". What a server does once loaded is the MCP stream's; this maps the
 * flags to the sources and the wait.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/mcp.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { SessionPlan } from './types.ts';

const call = coreModule('cli/flags/mcp');

/** "30 seconds by default." */
export const DEFAULT_MCP_TIMEOUT_MS: number = call('DEFAULT_MCP_TIMEOUT_MS');

/** The places MCP servers are read from. */
export const MCP_READ_SOURCES: readonly ['user', 'project', 'local', 'plugins', 'command-line'] =
  call('MCP_READ_SOURCES');
export type McpReadSource = (typeof MCP_READ_SOURCES)[number];

/**
 * The sources a session reads servers from: `--strict-mcp-config` and `--bare` leave the command
 * line's alone ("only servers supplied on the command line connect"). How a managed MCP file
 * combines with `--strict-mcp-config` is the MCP stream's (managed-mcp.md). Only the two fields it
 * reads cross to the core.
 */
export const mcpSourcesRead = (
  plan: Pick<SessionPlan, 'strictMcpConfig' | 'skipped'>,
): readonly McpReadSource[] =>
  call('mcpSourcesRead', { strictMcpConfig: plan.strictMcpConfig, skipped: plan.skipped });
