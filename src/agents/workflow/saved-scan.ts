/**
 * Reads the saved workflows (workflows.md, "Save the workflow for reuse") through the `FolderScan`
 * port: `<dir>/.claude/workflows/*.js` for the working directory and every parent up to the
 * repository root ("Project workflows also load from every `.claude/workflows/` along that path"), and
 * the personal `~/.claude/workflows/` (the `workflows/` folder under `CLAUDE_CONFIG_DIR` when set).
 * A workflow runs as `/<meta.name>`; a script without a readable `meta` is left out ("Claude Code
 * drops `/<name>` from `/` autocomplete"), with the issues of its `meta` at the file's path. A
 * workflow named `anthropic-skills` or `anthropic-skills:<name>` does not load (skills.md, "Names
 * reserved for synced skills"), with an error at its path.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/workflow/saved_scan.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { ScopeRequest } from '../../foundation/scope-folders.ts';
import type { FolderScan } from '../../ports/folder-scan.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { SavedResolution } from './saved.ts';

const call = coreModule('agents/workflow/saved-scan');

export interface SavedScanRequest extends ScopeRequest {
  readonly signal?: AbortSignal | undefined;
}

export interface SavedScan extends SavedResolution {
  readonly issues: readonly PluginIssue[];
}

export const scanSavedWorkflows = (
  folders: FolderScan,
  request: SavedScanRequest,
): Promise<SavedScan> => call.async('scanSavedWorkflows', folders, request);
