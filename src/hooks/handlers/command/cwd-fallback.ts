import type { PluginIssue } from '../../../format/issues.ts';
import { coreModule } from '../../../wasm/bridge.ts';
import type { RunContext } from '../context.ts';

/**
 * hooks.md, "Hook handler fields": "Handlers run in the current directory with Claude Code's
 * environment. If the current directory no longer exists, for example a worktree or temp directory
 * that another shell deleted mid-session, Claude Code runs command hooks from the first of these
 * that still exists: the directory the session started in, the project root, your home directory,
 * or the system temp directory. Claude Code records a warning naming the fallback directory."
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/handlers/command/cwd_fallback.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/handlers/command/cwd-fallback');

export interface WorkingDirectory {
  readonly cwd: string;
  readonly issues: readonly PluginIssue[];
}

export const workingDirectory = (context: RunContext): Promise<WorkingDirectory> =>
  call.async('workingDirectory', context);
