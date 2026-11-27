import type { PluginIssue } from '../format/issues.ts';
import type { InstallRecord } from './installed-plugins.ts';
import type { InstallScope } from './install-scopes.ts';
import type { StoreLayout } from './store-layout.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Which install records one `claude plugin uninstall` removes (plugins/troubleshooting.md, "A
 * plugin stays installed after `plugin uninstall` on Windows"): at project or local scope a record
 * belongs to a project folder, and "`installed_plugins.json` held two install records of the plugin
 * for the project folder, each spelling the folder's path differently, and one uninstall removes one
 * of them". The record under the spelling the command was run with goes first; "the second run finds
 * no record under its own spelling of the path, so it removes the one under the other spelling"
 * (from v2.1.295). Pure over the records; the host rewrites the file.
 *
 * Judgments (spec/judgments.md): "spelling" is the text of `projectPath` as written; the other
 * spelling is found by the platform's own comparison of paths (on Windows, capitals and the two
 * separators do not tell folders apart; elsewhere nothing but the text does); a project or local
 * uninstall that names no folder removes nothing and says so, since the scope is one folder's and
 * removing every project's record would uninstall the plugin everywhere; when no record matches
 * the command says `Plugin "<name>" is not installed in <scope> scope`; an uninstall marks the
 * version folders it leaves behind, inside the cache only (`dirsToOrphan`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/uninstall_records.rs`,
 * docs/RUST.md).
 */

const call = coreModule('marketplace/uninstall-records');

export interface UninstallSelector {
  /** What the command names, for its messages: `<name>` or `<name>@<marketplace>`. */
  readonly plugin: string;
  readonly layout: StoreLayout;
  readonly scope: InstallScope;
  /** The project folder the command ran in, as the command spells it; `project` and `local` scope only. */
  readonly projectPath?: string;
  readonly platform: 'posix' | 'windows';
}

export interface UninstallSelection {
  readonly removed: readonly InstallRecord[];
  readonly kept: readonly InstallRecord[];
  /** The version directories the uninstall leaves behind: write `.orphaned_at` into each. */
  readonly orphaned: readonly string[];
  readonly issues: readonly PluginIssue[];
}

export const selectUninstall = (
  records: readonly InstallRecord[],
  selector: UninstallSelector,
): UninstallSelection => call('selectUninstall', records, selector);
