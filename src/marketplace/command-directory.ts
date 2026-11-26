import type { PluginIssue } from '../format/issues.ts';
import type { PluginFiles } from '../ports/files.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * What a `command` source's printed directory must hold and stay within (plugins/marketplace-
 * reference.md, "Output that fails the install or update", "Copy mode and link mode"), judged
 * through a read-only view rooted at that directory.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/command_directory.rs`,
 * docs/RUST.md).
 */
const call = coreModule('marketplace/command-directory');

export interface CopyLimits {
  readonly maxBytes: number;
  readonly maxEntries: number;
}

/** The two ways a directory is too large to copy: the verdict, once a limit is broken. */
export const tooManyEntries = (maxEntries: number): PluginIssue =>
  call('tooManyEntries', maxEntries);

export const tooLarge = (maxBytes: number): PluginIssue => call('tooLarge', maxBytes);

export const contentIssue = (
  files: PluginFiles,
  signal: AbortSignal,
): Promise<PluginIssue | undefined> => call.async('contentIssue', files, signal);

/** In copy mode: the directory is at most 256 MiB with at most 20,000 entries. */
export const copyLimitIssue = (
  files: PluginFiles,
  signal: AbortSignal,
): Promise<PluginIssue | undefined> => call.async('copyLimitIssue', files, signal);

/** In link mode: "the install fails if a top-level entry is a symlink that points outside the printed directory". */
export const linkIssue = (
  files: PluginFiles,
  signal: AbortSignal,
): Promise<PluginIssue | undefined> => call.async('linkIssue', files, signal);
