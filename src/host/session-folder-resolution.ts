import type { ScannedEntry } from '../skills/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SessionDirectories } from './session-directories.ts';
import type { HostState } from './state.ts';
import type { FolderSkillSources } from './session-folders.ts';
import type { Entry } from './session-skill-entries.ts';

/**
 * The folder skills a session lists, resolved to their names.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_folder_resolution.rs`,
 * docs/RUST.md). The memo stays here: it compares the host's state object and the `taken` array
 * it was given.
 */

const call = coreModule('host/session-folder-resolution');

interface FolderResolution {
  readonly listed: readonly Entry[];
  readonly sources: FolderSkillSources;
}

/**
 * Memoized names and source-policy decisions, invalidated by scans, host.update and a change of
 * `taken` (a new array: the host's skills changed).
 */
export function createFolderResolution(
  stateOf: () => HostState,
  directories: SessionDirectories,
  taken: () => readonly string[],
) {
  let scanned: readonly ScannedEntry[] = [];
  let previous: { readonly state: HostState; readonly taken: readonly string[] } | undefined;
  let resolved: FolderResolution = { listed: [], sources: { entries: [], shadowed: [] } };
  const resolve = () => {
    const state = stateOf();
    const names = taken();
    if (previous?.state === state && previous.taken === names) return;
    resolved = call('resolveFolderEntries', scanned, state.customization.locked, {
      cwd: directories.cwd(),
      addedDirectories: directories.added(),
      taken: names,
    });
    previous = { state, taken: names };
  };
  return {
    set(entries: readonly ScannedEntry[]) {
      scanned = entries;
      previous = undefined;
      resolve();
    },
    current() {
      resolve();
      return resolved.listed;
    },
    sources() {
      resolve();
      return resolved.sources;
    },
  };
}
