/**
 * The one order of names the kit lists by, and a folder listed in that order.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/listing.rs`, docs/RUST.md).
 */
import type { DirectoryEntry, PluginFiles } from '../ports/files.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/listing');

/** Orders by UTF-16 code units, so an order never depends on the host's locale or its port. */
export const byCodeUnit = (first: string, second: string): number =>
  call('byCodeUnit', first, second);

/**
 * A folder's entries sorted by name (code units), or `undefined` when it does not exist. Every
 * listing the kit reads goes through here, so what a plugin loads, which duplicate wins and what
 * the skill listing keeps never depend on the order the host's `list` returns (judgment).
 */
export const listSorted = (
  files: Pick<PluginFiles, 'list'>,
  path: string,
  signal?: AbortSignal,
): Promise<readonly DirectoryEntry[] | undefined> => call.async('listSorted', files, path, signal);
