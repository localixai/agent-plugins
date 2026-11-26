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
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/listing.rs`, docs/RUST.md); only
 * the listing through the port stays here.
 */
export async function listSorted(
  files: Pick<PluginFiles, 'list'>,
  path: string,
  signal?: AbortSignal,
): Promise<readonly DirectoryEntry[] | undefined> {
  const entries = await files.list(path, signal);
  return entries?.toSorted((first, second) => byCodeUnit(first.name, second.name));
}
