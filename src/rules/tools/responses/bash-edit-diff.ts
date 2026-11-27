/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/responses/bash_edit_diff.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/responses/bash-edit-diff');

/**
 * One entry of `bashEditDiff.files` (hooks.md, "Bash"): the diff of a changed file "for display".
 * `hunks` is shown in the table and its shape is not documented, so it is kept as it came and is
 * not required (spec/judgments.md). The SDK's `BashOutput` has no `bashEditDiff` (the docs name
 * Claude Code v2.1.269), so nothing ties these types to the SDK.
 */
export interface BashEditFile {
  readonly filePath: string;
  readonly hunks?: unknown;
  /** `true` for a file the command added. */
  readonly created?: boolean | undefined;
  /** `true` for a file the command removed. */
  readonly deleted?: boolean | undefined;
}

/** `tool_response.bashEditDiff`: "best effort and in public beta", read field by field. */
export interface BashEditDiff {
  /** Absolute paths of the changed files, at most 200. */
  readonly changedFiles?: readonly string[] | undefined;
  /** Diffs of up to 5 changed files. */
  readonly files?: readonly BashEditFile[] | undefined;
  /** Count of changed files with no diff in `files`. */
  readonly moreFiles?: number | undefined;
  /** The diff is incomplete or could not be taken. */
  readonly unavailable?: boolean | undefined;
  /** A Git command that moves the working tree: no diff was taken. */
  readonly skipped?: boolean | undefined;
  /** Another Bash call ran in the same repository at the same time. */
  readonly shared?: boolean | undefined;
}

export interface BashEditDiffParse {
  /** `undefined` when the call carries no `bashEditDiff` or it could not be read. */
  readonly diff: BashEditDiff | undefined;
  readonly issues: readonly PluginIssue[];
}

/**
 * Reads `tool_response.bashEditDiff` of a Bash PostToolUse call (hooks.md, "Bash"): "the changed
 * files". A call without the field has no diff (background and read-only commands carry none, and
 * recording may be off); a wrong type is an error issue and nothing throws; a broken limit is a
 * warning and the data is kept whole.
 */
export const parseBashEditDiff = (toolResponse: unknown): BashEditDiffParse =>
  call('parseBashEditDiff', toolResponse);

/** How far a list can be relied on: "how complete and how reliable that list is". */
export interface BashEditDiffTrust {
  /** `false` when the diff is incomplete or could not be taken (`unavailable`), or was not taken (`skipped`). */
  readonly complete: boolean;
  /** `false` when another Bash call ran in the repository meanwhile, so some changes may be its (`shared`). */
  readonly attributable: boolean;
}

/** The two flags that say a list is incomplete and the one that says it may hold another command's changes. */
export const bashEditDiffTrust = (
  diff: Pick<BashEditDiff, 'unavailable' | 'skipped' | 'shared'>,
): BashEditDiffTrust => call('bashEditDiffTrust', diff);
