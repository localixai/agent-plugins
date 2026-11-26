/**
 * The `${VAR}` names a remote MCP server's `url` and `headers` read as empty, whether or not the
 * variable is set and whatever `:-default` it carries (mcp.md, "Credential variables that read as
 * empty"). The page lists them only "such as"; the set here is Claude Code's, observed with
 * the spec's remote-mcp-env oracle (spec/judgments.md, "Remote MCP credential names"), and
 * tests/loading/remote-mcp-credentials.test.ts checks it against the recorded result.
 *
 * Names compare without regard to case (`npm_token`, `Http_Proxy` are emptied too).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/remote_mcp_credentials.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/remote-mcp-credentials');

/** Whether a remote MCP server's `url` or `headers` reads `${name}` as empty. */
export const readsAsEmptyInRemoteMcp = (name: string): boolean =>
  call('readsAsEmptyInRemoteMcp', name);
