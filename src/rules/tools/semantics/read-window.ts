/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/semantics/read_window.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/semantics/read-window');

export interface ReadRequest {
  /** The line to start at, counting from 1. */
  readonly offset?: number | undefined;
  /** How many lines to read. */
  readonly limit?: number | undefined;
}

export interface NumberedLine {
  readonly number: number;
  readonly text: string;
}

export interface ReadSlice {
  readonly lines: readonly NumberedLine[];
  /** The lines in the whole file, whatever the window. */
  readonly lineCount: number;
  /** The notice that stands in for the lines: an empty file, or an offset past the last line. */
  readonly notice: string | undefined;
  readonly issues: readonly PluginIssue[];
}

/**
 * The window of a file the Read tool returns (tools-reference.md, "Read tool behavior"): the
 * contents "with line numbers", from the line `offset` ("line number to start reading from",
 * counted from 1: spec/judgments.md) for `limit` lines. An empty file and an `offset` past the last
 * line carry a notice instead of lines, the second giving the file's line count; the wording is a
 * judgment. The token and byte limits of large files are measured by the host.
 */
export const readWindow = (content: string, request: ReadRequest): ReadSlice =>
  call('readWindow', content, request);
