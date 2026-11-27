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

/** The findings of a parsed `.mcp.json`. */
export const mcpFindings = (raw: unknown, declared: ReadonlySet<string>): Finding[] =>
  call('mcpFindings', raw, declared);
