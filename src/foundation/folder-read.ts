import type { PluginIssue } from '../format/issues.ts';
import type { ScanFolder } from '../ports/folder-scan.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * A file of a scope folder read through the host's port, for the skills and the agents scans.
 * A port may refuse a file by rejecting (over a size limit, text that is not UTF-8, a file that
 * changed while it was read): the scan skips that file with a warning that carries the port's
 * message and goes on. An aborted `signal` still rejects, so a cancelled scan never turns into a
 * quiet partial answer (judgment, spec/judgments.md, "metadata-first catalog").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/foundation/folder_read.rs`, docs/RUST.md).
 */

const call = coreModule('foundation/folder-read');

export interface TextRead {
  /** `undefined` when the file is absent or the port refused it (then `issue` says why). */
  readonly text: string | undefined;
  readonly issue?: PluginIssue;
}

export const readTextOf = (
  folder: ScanFolder,
  path: string,
  signal?: AbortSignal,
): Promise<TextRead> => call.async('readTextOf', folder, path, signal);
