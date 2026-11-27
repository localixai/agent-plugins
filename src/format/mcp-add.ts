import { coreModule } from '../wasm/bridge.ts';
import type { McpAddRead } from './mcp-add-json.ts';
import type { EnvLookup } from './mcp-env.ts';

/**
 * `claude mcp add [options] <name> -- <command> [args...]` and `claude mcp add --transport <type>
 * <name> <url>` (mcp.md, "Option 1" to "Option 3"): the arguments become the entry `add-json` would
 * have received. Everything after `--` is the server's own command line, "passed to the server
 * untouched". The CLI itself, its help and its exit codes are the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_add.rs`, docs/RUST.md).
 */

const call = coreModule('format/mcp-add');

/**
 * Reads the arguments after `claude mcp add`. Judgments (spec/judgments.md): no `--transport` means
 * stdio; without `--` the command is the positional after the name; `--header` on a stdio server and
 * the OAuth flags on a stdio one have no effect and say so; `ws` is refused with a pointer to
 * `add-json`, since "the `claude mcp add --transport` flag doesn't accept `ws`"; the wording of every
 * refusal is the kit's.
 */
export const readMcpAdd = (
  argv: readonly string[],
  lookup: EnvLookup,
  existing: ReadonlySet<string> = new Set(),
): McpAddRead => call('readMcpAdd', argv, lookup, existing);
