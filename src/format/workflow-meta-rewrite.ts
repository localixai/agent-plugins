import type { PluginIssue } from './issues.ts';
import type { PhaseEntry } from './workflow-meta.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Rewrites single fields of the `meta` literal of a workflow script and keeps every other byte
 * (workflows.md, "Edit a saved script"). What the docs leave open (quoting, layout, line endings) is
 * in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/workflow_meta_rewrite.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/workflow-meta-rewrite');

/** The fields a rewrite sets; a field not given is left as written. */
export interface MetaPatch {
  readonly name?: string;
  readonly description?: string;
  readonly phases?: readonly PhaseEntry[];
  readonly whenToUse?: string;
}

/**
 * Sets fields of the `meta` literal and keeps every other byte of the script. A `meta` that is not a
 * literal is left alone (the reasons come back as issues), and so is a script the rewrite would leave
 * without a name or a description: `source` is then `undefined`.
 */
export const rewriteWorkflowMeta = (
  source: string,
  patch: MetaPatch,
): { readonly source: string | undefined; readonly issues: readonly PluginIssue[] } =>
  call('rewriteWorkflowMeta', source, patch);
