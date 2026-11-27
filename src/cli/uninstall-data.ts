import type { InstalledRecord } from '../marketplace/dependency-graph.ts';
import type { InstallScope } from '../marketplace/install-scopes.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('cli/uninstall-data');

/**
 * What `claude plugin uninstall` deletes and keeps (plugins/cli-reference.md, "plugin uninstall",
 * "What an uninstall deletes and keeps"). From the last scope a plugin is installed at, its stored
 * options and secrets and its data directory `~/.claude/plugins/data/<id>/` go, except: `--keep-data`
 * keeps the directory; another installed plugin that uses the same folder keeps it (the folder is
 * `pluginDataId`, and "an ID [that] differs from this one only in letter case" names the same
 * folder on a case-insensitive file system, so there folders are compared without regard to case,
 * J3.19); and installed plugins that cannot be read
 * back after the removal keep the options, the secrets and the directory ("the plugin may still be
 * installed at another scope"). Pure over what the host read back.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/uninstall_data.rs`, docs/RUST.md).
 */

/** Why the data directory stayed. `other-scope` is the judgment that a plugin still installed somewhere keeps everything. */
export type DataKeptBecause =
  | 'keep-data'
  | 'shared-folder'
  | 'other-scope'
  | 'install_records_unreadable';

/** "With `--json` the result carries `savedKept: \"install_records_unreadable\"`". */
export const SAVED_KEPT_UNREADABLE: 'install_records_unreadable' = call('SAVED_KEPT_UNREADABLE');

export interface RetentionFacts {
  /** `name@marketplace` of the plugin just removed from `scope`. */
  readonly id: string;
  readonly scope: InstallScope;
  readonly keepData?: boolean;
  /** The installed plugins read back after the removal from `scope`; `'unreadable'` when the list could not be read. */
  readonly remaining: readonly InstalledRecord[] | 'unreadable';
  /**
   * Whether the plugins root's file system ignores letter case (spec §3.1.2 item 4, J3.19): two ids
   * that differ only in case share one data directory exactly then. Default true, so a host that
   * cannot tell keeps a folder rather than deleting one another plugin may use.
   */
  readonly caseInsensitiveFileSystem?: boolean;
}

export interface Retention {
  /** Its stored options and secrets. */
  readonly saved: 'delete' | 'keep';
  readonly data: 'delete' | 'keep';
  /** Set when `saved` is `keep` because the records could not be read. */
  readonly savedKept?: typeof SAVED_KEPT_UNREADABLE;
  /** Set when `data` is `keep`. */
  readonly dataKeptBecause?: DataKeptBecause;
  /** The folder name under `~/.claude/plugins/data/`. */
  readonly dataFolder: string;
}

/** What the uninstall of `facts.id` from `facts.scope` deletes and keeps. */
export const uninstallRetention = (facts: RetentionFacts): Retention =>
  call('uninstallRetention', facts);

/** `/plugin` "shows `· data preserved` when it did". */
export const DATA_PRESERVED: string = call('DATA_PRESERVED');
