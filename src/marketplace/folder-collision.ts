import { coreModule } from '../wasm/bridge.ts';
import type { InstallSurface } from './install-errors.ts';

/**
 * Two plugin ids that share one folder (plugins/troubleshooting.md, "`\"<plugin>\" was not installed:
 * it would share its folder with \"<other>\"`"): "the same once `.` and `@` are written as `-`. On
 * macOS and Windows, ids that differ only in capitals map to the same folder too. Installing both
 * would put one plugin's files in the other's folder, so Claude Code refuses and the installed
 * plugin keeps its files." Pure over ids; the host reads the installed ones and refuses the install.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/folder_collision.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/folder-collision');

export type FolderPlatform = 'macos' | 'windows' | 'linux';

/** The folder name an id maps to: `.` and `@` written as `-`, capitals folded where the platform does. */
export const folderOf = (id: string, platform: FolderPlatform): string =>
  call('folderOf', id, platform);

/** What the two plugins would share: the section quotes both `its folder` and `its saved data`. */
export type SharedThing = 'folder' | 'saved data';

export interface FolderCollision {
  /** The id that is refused. */
  readonly plugin: string;
  /** The id it would share a folder with. */
  readonly other: string;
  /** The other plugin is installed, so uninstalling it is a way out; false when both arrive in one install. */
  readonly otherInstalled: boolean;
  readonly message: string;
}

/**
 * The incoming ids that cannot be installed: those that map to the folder of an installed id, and
 * those that map to the folder of an id that arrived earlier in the same install (judgment: the
 * later one is refused). An id that is already installed is not a collision with itself.
 */
export const folderCollisions = (input: {
  readonly incoming: readonly string[];
  readonly installed: readonly string[];
  readonly platform: FolderPlatform;
  readonly surface: InstallSurface;
  /** Judgment: the docs print `folder` or `saved data` without saying when; the caller names the one it checked. */
  readonly shares?: SharedThing;
}): readonly FolderCollision[] => call('folderCollisions', input);
