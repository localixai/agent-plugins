import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { StagedCommit, StagedDirectory } from './ports.ts';

/**
 * Installing a fetched plugin by moving the directory the host staged it in into the version folder
 * of the cache (plugins/troubleshooting.md, "Could not move the new copy of this plugin version
 * into <path>": "Claude Code downloads a fresh copy of its files and moves it into that version's
 * folder in the plugin cache"). The move is the host's, atomic and with the earlier copy set aside
 * and put back; a move that fails is the documented message, never a throw. A staged tree that
 * holds symbolic links is installed with the `TreePlan` of `planTreeCopy`, which the port applies;
 * a plain move would ship the links that point outside the marketplace and could not dereference
 * the ones inside it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/staged_commit.rs`,
 * docs/RUST.md).
 */
const call = coreModule('marketplace/staged-commit');

/**
 * Installs `stagedPath` at `destination` (moved whole, or materialized by `plan`). `undefined` on success; otherwise the issue that says why it
 * is not installed, with what became of the copy installed before. An abort is an issue of its own:
 * nothing moved, so there is no earlier copy to report.
 */
export const commitStagedPlugin = (
  input: StagedCommit & {
    readonly staged: StagedDirectory;
    readonly signal: AbortSignal;
  },
): Promise<PluginIssue | undefined> => call.async('commitStagedPlugin', input);
