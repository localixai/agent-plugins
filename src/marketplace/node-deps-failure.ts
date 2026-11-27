import type { PluginIssue } from '../format/issues.ts';
import type { CommandResult } from '../ports/command-runner.ts';
import type { NodeDepsPlan, SkipOf, SkipReason } from './node-deps-types.ts';
import type { ReadPackageJson } from './node-deps-spec.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What the user sees when the dependency install fails or cannot run, and that the plugin still
 * loads (plugins/loading.md, "When the dependency install fails or is skipped": "the plugin still
 * loads, but the parts of it that need the missing packages may not work"; plugins/
 * troubleshooting.md, "The packages it lists are not installed" / "were not installed, because
 * ..."). The docs give the two openings of the note, the command it gives and the example reasons;
 * the words that join them are the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/node_deps_failure.rs`,
 * docs/RUST.md).
 */

const call = coreModule('marketplace/node-deps-failure');

// ---- the reasons an install cannot run ----

/**
 * Why an install cannot run, as the clause that follows "because" in the note (`it has no
 * package.json`): for a host that words its own message, such as `Node.js packages were not
 * installed: <clause>`.
 */
export const skipReasonText = <R extends SkipReason>(skip: SkipOf<R>): string =>
  call('skipReasonText', skip);

// ---- the note ----

export type NoteKind = 'not-installed' | 'not-installed-because';

export interface NodeDepsNote {
  readonly kind: NoteKind;
  /** The note `/plugin` and `claude plugin list` show. */
  readonly text: string;
  /** "run the `claude plugin update` command that the note gives"; absent when updating cannot help. */
  readonly updateCommand?: string;
  /** What to do when the automatic install cannot provide the packages. */
  readonly advice?: string;
  /** The note as a finding: a warning, because the plugin still loads. */
  readonly issues: readonly PluginIssue[];
}

/** The cached copy of an enabled plugin, as far as the note goes. */
export interface CachedCopy {
  readonly enabled: boolean;
  /** Any lockfile is at the plugin root, supported or not. */
  readonly hasLockfile: boolean;
  readonly packageJson: ReadPackageJson | undefined;
  readonly hasNodeModules: boolean;
}

/**
 * The note on an enabled plugin whose cached copy has a lockfile and a `package.json` that lists
 * runtime dependencies, but no `node_modules`: `are not installed` when the install can run and did
 * not finish, `were not installed, because ...` when it cannot run. `plan` is what
 * `planNodeDepsInstall` says for the copy now; none for a plugin that has no cached copy.
 */
export const nodeDepsNote = (
  copy: CachedCopy,
  plan: NodeDepsPlan,
  id: string,
): NodeDepsNote | undefined => call('nodeDepsNote', copy, plan, id);

/**
 * "While `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is set, `claude plugin update` and `/plugin`
 * skip the retry and report that the plugin is at its latest version."
 */
export const updateRetriesInstall = (
  shown: NodeDepsNote,
  nonessentialTrafficDisabled: boolean,
): boolean => call('updateRetriesInstall', shown, nonessentialTrafficDisabled);

// ---- a run that failed ----

export type NodeDepsFailureCause = 'did-not-start' | 'timed-out' | 'aborted' | 'exit-code';

export type InstallOutcome =
  | { readonly state: 'installed' }
  | { readonly state: 'failed'; readonly cause: NodeDepsFailureCause; readonly message: string };

/** What a finished run means: installed, or failed with its cause and the end of its output. */
export const installOutcome = (result: CommandResult): InstallOutcome =>
  call('installOutcome', result);
