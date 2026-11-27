/**
 * Where Claude Code looks for the settings files and for output styles (settings.md, "Where Claude
 * Code keeps the local file in a git repository"; permissions.md, "Where output styles, hooks, and
 * local settings load from"). Pure over paths the host knows: the kit reads no file and joins paths
 * through the host's own `join` (`Locations.joinPath`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/locations.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('settings/locations');

export interface SettingsLocationsInput {
  /** The session's primary working directory: where `.claude/settings.json` is read from. */
  readonly primaryWorkingDirectory: string;
  /** `~`. */
  readonly home: string;
  /** The `CLAUDE_CONFIG_DIR` directory, which replaces `~/.claude`; empty counts as unset. */
  readonly configDirectory?: string;
  /** The working directory and each of its parents, nearest first, up to the filesystem root. */
  readonly directories: readonly string[];
  /** The root of the git repository, the main checkout's root in a worktree; absent outside git. */
  readonly repositoryRoot?: string;
  /** A Windows host. */
  readonly windows: boolean;
  /** The repository root or its `.git` or `.claude` entry is owned by another user. */
  readonly repositoryNotOwned?: boolean;
  /** `base` and a `/`-separated relative path, joined the way the host writes paths. */
  readonly join: (base: string, relativePath: string) => string;
}

export interface SettingsLocations {
  /** `~/.claude/settings.json`, or `$CLAUDE_CONFIG_DIR/settings.json`: every project on the machine. */
  readonly user: string;
  /** `.claude/settings.json` of the primary working directory, with no parent-directory fallback. */
  readonly project: string;
  /** `.claude/settings.local.json`, at the repository root unless it stays with the shared file. */
  readonly local: string;
  /**
   * The folders output styles are discovered in, nearest first: "the current working directory and
   * its parents, your user directory at `~/.claude/`" (managed settings add theirs elsewhere).
   */
  readonly outputStyles: readonly string[];
}

export const settingsLocations = (input: SettingsLocationsInput): SettingsLocations =>
  call('settingsLocations', input);
