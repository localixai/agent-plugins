/**
 * One server of a core `mcp.json` (core §7.2.1): an object with a `type` that matches exactly one of
 * three closed variants. An unknown field, an unknown `type` or a field of another variant makes the
 * entry invalid; each variant takes an optional `extensions` (core §8.1). The rules are the schema's
 * (`schemas/1.1.0/mcp.schema.json`, `#/$defs/server`) with the text's: the command token, the `cwd`
 * forms and containment, the reserved `env` names (core §9.2), the URL and header rules.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/mcp_server.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { CoreExtensions } from './extensions.ts';

const call = coreModule('format/core/mcp-server');

export const CORE_TRANSPORTS: readonly ['stdio', 'streamable-http', 'sse'] =
  call('CORE_TRANSPORTS');
export type CoreTransport = (typeof CORE_TRANSPORTS)[number];

export interface CoreStdioServer {
  readonly type: 'stdio';
  readonly command: string;
  readonly args?: readonly string[];
  readonly env?: Readonly<Record<string, string>>;
  readonly cwd?: string;
  readonly extensions?: CoreExtensions;
}

export interface CoreRemoteServer {
  readonly type: 'streamable-http' | 'sse';
  readonly url: string;
  readonly headers?: Readonly<Record<string, string>>;
  readonly extensions?: CoreExtensions;
}

export type CoreMcpServer = CoreStdioServer | CoreRemoteServer;

/** The entry as a server, or why it is invalid (core §7.2.2 item 3: the server is skipped). */
export const readCoreServer = (
  raw: unknown,
  implemented: readonly string[],
): { readonly server: CoreMcpServer } | { readonly problem: string } =>
  call('readCoreServer', raw, implemented);
