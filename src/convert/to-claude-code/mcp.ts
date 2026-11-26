import type { CoreMcpServer } from '../../format/core/mcp-server.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ConversionNote } from '../types.ts';

/**
 * `.mcp.json` from a core `mcp.json` (Agent Plugins Extended §18.4.2 item 3, the table of §18.3.3):
 * `stdio` stays, `streamable-http` is `http`, `sse` stays; a `./` command is
 * `${CLAUDE_PLUGIN_ROOT}/…`; `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` become Claude Code's names, and a
 * stdio server gets `PLUGIN_ROOT` and `PLUGIN_DATA` in `env` (J18.18). `cwd` and `extensions` are
 * left out and reported; a server with a `${` left in `args`, `env`, `url` or `headers`, which Claude
 * Code would expand as a variable (§8.4.2) with no escape, is not written. Entries the core skipped
 * are not in the package, so they are not written either.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/to_claude_code/mcp.rs`,
 * docs/RUST.md).
 */
const call = coreModule('convert/to-claude-code/mcp');

export const CLAUDE_MCP: string = call('CLAUDE_MCP');

type Written = Readonly<Record<string, unknown>>;

export interface McpPart {
  readonly servers: Readonly<Record<string, Written>>;
  readonly notes: readonly ConversionNote[];
}

export const claudeCodeMcp = (servers: Readonly<Record<string, CoreMcpServer>>): McpPart =>
  call('claudeCodeMcp', servers);
