import { coreModule } from '../../wasm/bridge.ts';
import type { Finding } from './finding.ts';

/**
 * The MCP server checks of `claude plugin validate` on `.mcp.json` (oracle 2.1.319; the changelog's
 * "reports `.mcp.json` entries that would be silently dropped at load, undeclared
 * `${user_config.*}` references, and insecure URLs"). The file is a `mcpServers` wrapper or the bare
 * map of servers; each entry is read on its own.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/mcp_check.rs`, docs/RUST.md).
 */

const call = coreModule('format/validate/mcp-check');

/** Only what the manifest's own schema leaves to the MCP check: undeclared references, urls and headers. */
export const inlineFindings = (
  servers: Readonly<Record<string, unknown>>,
  declared: ReadonlySet<string>,
): Finding[] => call('inlineFindings', servers, declared);

/** The findings of a parsed `.mcp.json`. */
export const mcpFindings = (raw: unknown, declared: ReadonlySet<string>): Finding[] =>
  call('mcpFindings', raw, declared);
