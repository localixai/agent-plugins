import type { PluginIssue } from './issues.ts';
import type { WorkflowMeta } from './workflow-meta.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What the kit can say about a workflow script without running it (workflows.md, "Edit a saved
 * script", "Behavior and limits"): the stage titles `phase()` is called with against `meta.phases`,
 * whether the body reads `args`, the calls that throw inside the script (`Date.now()`,
 * `Math.random()`, a no-argument `new Date()`) and `import()`, which fails before the run starts.
 * Comments, strings, template text and regular expressions are never code. A syntax error in the body
 * is the runtime's to report ("Claude Code reports it when you run the workflow"), not the kit's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/workflow_script.rs`, docs/RUST.md).
 */

const call = coreModule('format/workflow-script');

export interface PhaseReport {
  /** The titles of `meta.phases`. */
  readonly declared: readonly string[];
  /** The titles `phase()` is called with as a literal, once each, in order. */
  readonly used: readonly string[];
  /** Used and not declared: "a `phase()` title with no entry gets a progress group of its own". */
  readonly ownGroup: readonly string[];
  /** Declared and never used. */
  readonly unused: readonly string[];
}

export interface ScriptReport {
  readonly meta: WorkflowMeta | undefined;
  readonly issues: readonly PluginIssue[];
  /** The script up to the end of the `meta` statement; `header + body` is the script. */
  readonly header: string;
  readonly body: string;
  readonly phases: PhaseReport;
  /** The body reads the `args` global (workflows.md, "Pass input to a saved workflow"). */
  readonly readsArgs: boolean;
}

/** Inspects a workflow script. Never runs it and never throws. */
export const inspectWorkflowScript = (source: string): ScriptReport =>
  call('inspectWorkflowScript', source);
