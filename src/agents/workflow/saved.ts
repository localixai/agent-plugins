/**
 * Where a saved workflow is written and which one `/<name>` runs (workflows.md, "Save the workflow
 * for reuse"). Pure over the facts the host reads from the disk; the host does the write and the
 * `lstat`s.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/workflow/saved.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('agents/workflow/saved');

export const SAVE_SCOPES: readonly ['project', 'personal'] = call('SAVE_SCOPES');
export type SaveScope = (typeof SAVE_SCOPES)[number];

export interface SaveRequest {
  readonly scope: SaveScope;
  /** The workflow's file name without `.js`. */
  readonly name: string;
  /** The working directory, then each parent up to the repository root, nearest first (`FolderScan.repository`). */
  readonly directories: readonly string[];
  /** The directories of that chain that already have a `.claude/workflows` folder. */
  readonly withWorkflows: ReadonlySet<string>;
  readonly home: string;
  /** `CLAUDE_CONFIG_DIR`, when set. */
  readonly configDir?: string | undefined;
  /** `Locations.joinPath`: a base and a `/`-separated relative path, joined the way the host writes paths. */
  readonly join: (base: string, relativePath: string) => string;
}

export interface SavePlan {
  readonly scope: SaveScope;
  /** The folder the file goes in. */
  readonly directory: string;
  /** The file: `<directory>/<name>.js`. */
  readonly path: string;
  /** The paths that must not be symbolic links, for the host to check before it writes. */
  readonly mustNotBeLinks: readonly string[];
}

export type SaveOutcome =
  | { readonly plan: SavePlan; readonly issue?: undefined }
  | { readonly plan?: undefined; readonly issue: PluginIssue };

/** The plan of a save, or the issue that refuses it. */
export const planWorkflowSave = (request: SaveRequest): SaveOutcome =>
  call('planWorkflowSave', request);

/** "shows an error instead of writing through one": the first of `mustNotBeLinks` that is a symbolic link. */
export const refuseLinks = (plan: SavePlan, linked: ReadonlySet<string>): PluginIssue | undefined =>
  call('refuseLinks', plan, linked);

/** A workflow file found in a save location. */
export interface SavedWorkflow {
  /** What `/<name>` runs it as: its `meta.name`. */
  readonly name: string;
  /**
   * Its `meta.description`, which the `meta` block must hold (workflows.md, "Edit a saved script"),
   * for a listing beside the name. Always set by `scanSavedWorkflows`; optional so a host that builds
   * the list itself need not give one.
   */
  readonly description?: string;
  readonly scope: SaveScope;
  /** For a project workflow, how many levels above the working directory its `.claude/workflows` is: 0 is the working directory's own. */
  readonly distance: number;
  readonly path: string;
}

export interface SavedResolution {
  /** The workflow each name runs. */
  readonly winners: readonly SavedWorkflow[];
  /** The ones a winner of the same name hides. */
  readonly shadowed: readonly SavedWorkflow[];
}

/**
 * "when more than one defines the same name Claude Code runs the one closest to the working
 * directory. If a project workflow and a personal workflow share a name, the project one runs." Two in
 * one folder with one name: the first path in code-unit order (judgment).
 */
export const resolveSavedWorkflows = (found: readonly SavedWorkflow[]): SavedResolution =>
  call('resolveSavedWorkflows', found);
