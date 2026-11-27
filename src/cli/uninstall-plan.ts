import type { GraphFacts, InstalledRecord } from '../marketplace/dependency-graph.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput } from './result.ts';
import type { ConfigScope } from './scope.ts';
import type { StillSwitchedOn } from './uninstall-refusal.ts';

/**
 * `claude plugin uninstall <plugin>` as two decisions around the host's writes (plugins/
 * cli-reference.md, "plugin uninstall"): `startUninstall` reads the arguments and the installed
 * plugins and says what to remove, or what the command ends with; the host then switches the plugin
 * off in the scope's settings, removes its install record and reads the installed plugins back, and
 * `concludeUninstall` says what the command prints and which saved state (options, secrets, data
 * directory) goes. The retention rules are `uninstallRetention`'s (G8). What the docs leave open is
 * in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/uninstall_plan.rs`, docs/RUST.md).
 */

const call = coreModule('cli/uninstall-plan');

export interface UninstallArguments {
  /** `name` or `name@marketplace`. */
  readonly plugin: string;
  readonly scope?: string;
  readonly keepData?: boolean;
  readonly prune?: boolean;
  readonly yes?: boolean;
  readonly json?: boolean;
}

export interface UninstallFacts {
  /** What is installed, at every scope. */
  readonly graph: GraphFacts;
  /** Both stdin and stdout are a terminal. */
  readonly tty: boolean;
  /** What `readInstalledPlugins` reported for records it cannot read: the command refuses (§16.6.2). */
  readonly unreadableRecords?: string;
}

/** What `--prune` removes with the plugin, and whether the user is asked first. */
export interface PruneStep {
  /** The auto-installed dependencies no remaining plugin needs. */
  readonly orphans: readonly string[];
  /** `skip`: `-y`; `ask`: the host asks `Remove? [y/N]` and reports the answer; `none`: nothing to remove. */
  readonly confirm: 'none' | 'skip' | 'ask';
}

export type UninstallStart =
  | { readonly kind: 'done'; readonly output: CommandOutput }
  | {
      readonly kind: 'remove';
      /** The plugin as the user typed it, for the failure line. */
      readonly argument: string;
      readonly id: string;
      readonly scope: ConfigScope;
      readonly keepData: boolean;
      readonly json: boolean;
      readonly prune: PruneStep;
      /** The enabled plugins that depend on it, by name: not refused, the line ends `— warning: required by a, c` (oracle 2.1.322). */
      readonly requiredBy: readonly string[];
    };

/** What the host found after switching the plugin off and removing its record. */
export interface AfterRemoval {
  /** The settings file that still switches, or may still switch, the plugin on at the scope. */
  readonly settingsStillOn?: StillSwitchedOn;
  /** The installed plugins read back; `'unreadable'` when the list could not be read. */
  readonly remaining: readonly InstalledRecord[] | 'unreadable';
  /** The answer to `Remove? [y/N]`, when `prune.confirm` was `ask`. */
  readonly pruneConfirmed?: boolean;
}

/** State the host deletes for one uninstalled plugin: only what the retention rules let go. */
export interface Cleanup {
  readonly id: string;
  /** Its stored options and secrets. */
  readonly deleteSaved: boolean;
  /** The folder under `~/.claude/plugins/data/` to delete; absent when the directory stays. */
  readonly deleteDataFolder?: string;
}

export interface UninstallConclusion {
  readonly output: CommandOutput;
  readonly json: boolean;
  /** Whether the host removes the install record (not when the settings still switch it on). */
  readonly removeRecord: boolean;
  /** The state to delete, one entry per plugin that has any. */
  readonly cleanup: readonly Cleanup[];
  /** The orphaned dependencies the host also uninstalls from the scope. */
  readonly removePruned: readonly string[];
}

/** What to remove, or what the command ends with before anything is touched. */
export const startUninstall = (args: UninstallArguments, facts: UninstallFacts): UninstallStart =>
  call('startUninstall', args, facts);

/** What the command prints and which state goes, once the host has switched the plugin off and removed its record. */
export const concludeUninstall = (
  start: Extract<UninstallStart, { kind: 'remove' }>,
  after: AfterRemoval,
): UninstallConclusion => call('concludeUninstall', start, after);

/** What `/plugin` shows beside a plugin whose data directory stayed (`keptData`). */
export const dataPreservedMark = (keptData: boolean): string => call('dataPreservedMark', keptData);
