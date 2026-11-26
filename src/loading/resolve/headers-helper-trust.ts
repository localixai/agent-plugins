/**
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/headers_helper_trust.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/resolve/headers-helper-trust');

/**
 * How a folder came to be trusted (mcp.md, "Trust a folder before its headersHelper runs"). Only the
 * folder's own persisted trust counts: "a parent folder's trust, and the automatic trust a `claude -p`
 * or SDK session gets for hooks in settings files" do not. The host says which it has; the kit reads
 * no dialog and no `~/.claude.json` but through `persistedTrust`.
 */
export const TRUST_SOURCES: readonly ['persisted', 'parent-folder', 'automatic-session', 'none'] =
  call('TRUST_SOURCES');
export type TrustSource = (typeof TRUST_SOURCES)[number];

export interface HelperTrust {
  /** How the folder the server is declared in came to be trusted. */
  readonly source: TrustSource;
  /** The folder trust is keyed on (permissions.md, "Project allow rules and workspace trust"). */
  readonly key: string;
  /** An interactive session; the refusal goes to the debug log there and to stderr otherwise. */
  readonly interactive: boolean;
}

/**
 * "Set `projects["<path>"].hasTrustDialogAccepted` to `true` in `~/.claude.json`": that folder's
 * persisted trust, read from the parsed file. Another path's entry, a parent's, a non-`true` value
 * and a file of another shape are no trust.
 */
export const persistedTrust = (claudeJson: unknown, key: string): TrustSource =>
  call('persistedTrust', claudeJson, key);

export interface HelperNotice {
  readonly text: string;
  /** "Claude Code writes this line in non-interactive mode only"; an interactive session logs it. */
  readonly channel: 'stderr' | 'debug-log';
}
