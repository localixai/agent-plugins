/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/encoding.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/encoding');

/** errors.md, "File is not valid UTF-8": the message the tool result carries, verbatim. */
export const NOT_UTF8_MESSAGE: string = call('NOT_UTF8_MESSAGE');

/**
 * A tool that edits in place and refuses a file that is not valid UTF-8 (tools-reference.md,
 * "Non-UTF-8 files"); Edit still takes one that starts with a little-endian UTF-16 byte-order mark.
 */
export type InPlaceEditTool = 'Edit' | 'NotebookEdit';

/**
 * The refusal of an Edit or NotebookEdit on a file whose bytes do not decode as UTF-8, or
 * `undefined` when the tool may edit it. The check covers the bytes as a whole: one invalid
 * sequence is enough (errors.md).
 */
export const editEncodingIssue = (
  bytes: Uint8Array,
  tool: InPlaceEditTool,
): PluginIssue | undefined => call('editEncodingIssue', bytes, tool);

/**
 * The refusal of a Write that would replace such a file with text holding `U+FFFD`
 * (tools-reference.md, "Non-UTF-8 files": "Claude can still replace such a file with Write, unless
 * the new content contains `U+FFFD`, the character Read shows Claude in place of bytes it can't
 * decode. That guard stops Claude from writing back the garbled text it read."). `existing` is the
 * file's bytes, `undefined` for a file that is not there. "Such a file" is one `editEncodingIssue`
 * refuses for Edit; the wording of this refusal is a judgment (spec/judgments.md).
 */
export const writeEncodingIssue = (
  existing: Uint8Array | undefined,
  content: string,
): PluginIssue | undefined => call('writeEncodingIssue', existing, content);
