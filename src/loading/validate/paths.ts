import { coreModule } from '../../wasm/bridge.ts';
import type { DirectoryEntry, PluginFiles } from '../../ports/index.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/paths.rs`, docs/RUST.md). */
const call = coreModule('loading/validate/paths');

/** A manifest path (`./skills/a`, `./`) as a `PluginFiles` path: no `./`, no trailing slash, `.` for the root. */
export const pluginPath = (path: string): string => call('pluginPath', path);

/** `name` below `dir`; a name that is `.` is `dir` itself. */
export const childOf = (dir: string, name: string): string => call('childOf', dir, name);

/** A folder's entries, or none when it does not exist. */
export const entriesOf = (
  files: PluginFiles,
  dir: string,
  signal?: AbortSignal,
): Promise<readonly DirectoryEntry[]> => call.async('entriesOf', files, dir, signal);
