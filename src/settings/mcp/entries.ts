/**
 * The entries of `allowedMcpServers` and `deniedMcpServers` (managed-mcp.md, "Match servers by URL,
 * command, or name"): "an object with a single key that identifies servers by their URL, their command,
 * or their name". The two lists differ in one rule, the `serverName` text: the allowlist limits it to
 * "letters, numbers, hyphens, and underscores", the denylist takes "any non-empty string without
 * leading or trailing whitespace", so a claude.ai connector can be blocked by its display name.
 *
 * Invalid entries follow managed-settings.md, "Keys that fail closed": "an individual invalid entry is
 * stripped and the valid subset is enforced". A value that is no list: unreadable in the policy tier an
 * allowlist is enforced as an empty one ("so no MCP servers that users add are admitted"); a denylist
 * is dropped with a warning ("denying every server would block servers the policy never named"); in
 * any other source the value is dropped.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/entries.rs`, docs/RUST.md).
 */
import type { Repair } from '../repair/support.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/entries');

export const MCP_ENTRY_KINDS: readonly ['serverName', 'serverCommand', 'serverUrl'] =
  call('MCP_ENTRY_KINDS');
export type McpEntryKind = (typeof MCP_ENTRY_KINDS)[number];

export type McpEntry =
  | { readonly kind: 'serverName'; readonly value: string }
  | { readonly kind: 'serverCommand'; readonly value: readonly string[] }
  | { readonly kind: 'serverUrl'; readonly value: string };

export const MCP_LISTS: readonly ['allowed', 'denied'] = call('MCP_LISTS');
export type McpList = (typeof MCP_LISTS)[number];

/** The entry `raw` is, or `undefined` when it is not an object with exactly one valid key. */
export const readMcpEntry = (raw: unknown, list: McpList): McpEntry | undefined =>
  call('readMcpEntry', raw, list);

/** The valid entries of a list value, in order; what is no list has none. */
export const readMcpEntries = (value: unknown, list: McpList): readonly McpEntry[] =>
  call('readMcpEntries', value, list);

export const allowedMcpServers: Repair = (value, at) => call('allowedMcpServers', value, at);
export const deniedMcpServers: Repair = (value, at) => call('deniedMcpServers', value, at);
