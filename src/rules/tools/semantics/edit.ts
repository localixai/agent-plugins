/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/edit.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/edit');

/** The `old_string`, `new_string` and `replace_all` of an Edit call (hooks.md, "Edit"). */
export interface EditRequest {
  readonly old_string: string;
  readonly new_string: string;
  readonly replace_all?: boolean | undefined;
}

export interface EditResult {
  /** The file after the edit; the file as it was when `issues` is not empty. */
  readonly content: string;
  readonly replacements: number;
  readonly issues: readonly PluginIssue[];
}

/**
 * The Edit tool's replacement (tools-reference.md, "Edit tool behavior"): exact string replacement,
 * no regex and no fuzzy matching. `old_string` must be in the file as written (Match) and appear
 * exactly once unless `replace_all` is true (Uniqueness). `new_string` is inserted as plain text,
 * so `$&` stays `$&`. An empty `old_string` has nothing to find and is refused (spec/judgments.md),
 * as are the wordings of the refusals. Reading and writing the file, and the read-before-edit
 * check, are the host's.
 */
export const applyEdit = (content: string, request: EditRequest): EditResult =>
  call('applyEdit', content, request);
