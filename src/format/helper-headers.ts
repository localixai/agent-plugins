/**
 * What a `headersHelper` prints (mcp.md, "Use dynamic headers for custom authentication": "a JSON
 * object of string key-value pairs to stdout"; plugins/host-marketplace.md, "Write the headersHelper
 * command"): the one reader for an MCP server's helper and a marketplace's. Judgment
 * (spec/judgments.md): the whole of stdout is parsed, and one value that is not a string refuses
 * the output, since the docs name no partial reading.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/helper_headers.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/helper-headers');

export type HelperHeadersRead =
  | { readonly headers: Record<string, string> }
  | { readonly problem: 'not-an-object' | 'non-string-value' };

export const readHelperHeaders = (stdout: string): HelperHeadersRead =>
  call('readHelperHeaders', stdout);

/** Whether a header set holds `Authorization`, in any letter case. */
export const hasAuthorizationHeader = (headers: Readonly<Record<string, string>>): boolean =>
  call('hasAuthorizationHeader', headers);
