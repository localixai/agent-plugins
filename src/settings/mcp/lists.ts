/** Implemented by the Rust core (`crates/agent-plugins/src/settings/mcp/lists.rs`, docs/RUST.md). */
import type { PolicyView } from '../view.ts';
import type { McpEntry, McpEntryKind } from './entries.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/mcp/lists');

/** The entries of an allowlist, by the key they use: a server is matched against the entries of its own kind. */
export type McpEntriesByKind = { readonly [K in McpEntryKind]: readonly McpEntry[] };

export interface McpLists {
  /** The denylist: every file's entries, "regardless" of any lock. */
  readonly denied: readonly McpEntry[];
  /** The allowlist in force; `undefined` when no source sets one, an empty one when it is set empty. */
  readonly allowed: McpEntriesByKind | undefined;
}

/**
 * The two lists over the view. The allowlist in force is the key row's (`KEY_SPECS.allowedMcpServers`):
 * without `allowManagedMcpServersOnly` "allowlists from every settings scope merge", a managed list
 * blocking the parent's; under the lock only the list "from the highest-ranked admin source that sets
 * one" counts, else the parent's (managed-settings.md, "Let an embedding host add policy").
 */
export const readMcpLists = (view: PolicyView): McpLists => call('readMcpLists', view);
