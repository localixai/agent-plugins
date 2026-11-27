import { coreModule } from '../wasm/bridge.ts';
import type { SessionOptions } from './session-types.ts';

/**
 * The directories of a live session: the working directory and the directories added with
 * `/add-dir`. `SessionOptions` gives the first values; `/cd` (permissions.md, "Move the session to
 * another directory") and `/add-dir` change them while the session, its hooks and its `once` keys go
 * on. Every part of the session reads them through here, never from the options.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_directories.rs`, docs/RUST.md).
 */
export interface SessionDirectories {
  cwd(): string;
  /** The added directories, in the order they were added. */
  added(): readonly string[];
  /** `/add-dir`: `false` when the directory was already added or is the working directory. */
  add(path: string): boolean;
  /** `/cd`. */
  moveTo(cwd: string): void;
}

const call = coreModule('host/session-directories');

export const createSessionDirectories = (options: SessionOptions): SessionDirectories =>
  call('createSessionDirectories', options);
