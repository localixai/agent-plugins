/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/responses/structured_patch.rs`,
 * docs/RUST.md).
 */
import type { FileEditOutput } from '../../../protocol/official-tools.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/responses/structured-patch');

/** One hunk of an `Edit` or `Write` result's `structuredPatch` (the SDK's `FileEditOutput`). */
export type PatchHunk = FileEditOutput['structuredPatch'][number];

/**
 * The hunks of a unified diff as the `structuredPatch` of an `Edit` or `Write` result
 * (agent-sdk/typescript.md, "Edit" and "Write" output: "Diff hunks"; empty when nothing changed).
 * The file headers (`---`, `+++`, `Index:`) are skipped, and so is any text that is no hunk.
 * Judgment: the docs name the field and its type and not how a host derives it from its own diff
 * (spec/judgments.md).
 */
export const structuredPatchOf = (diff: string): readonly PatchHunk[] =>
  call('structuredPatchOf', diff);
