import type { FolderScan } from '../ports/folder-scan.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The project directories whose `.claude` folder is read: the working directory and every parent up
 * to the repository root, nearest first. In a linked worktree whose top has no `.claude/skills`, the
 * main checkout's loads as well, last (v2.1.277 and later; skills.md, "Load skills in monorepos and
 * subdirectories"). Judgment (spec/judgments.md): worktrees.md titles that bullet "skills, agents, and
 * commands", so the same rule gives the agents and commands folders, and the main checkout is never
 * a base of a skill-folder plugin. Shared by the skills and the agents scans.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/foundation/project_directories.rs`, docs/RUST.md).
 */

const call = coreModule('foundation/project-directories');

export const projectDirectories = (
  folders: FolderScan,
  cwd: string,
  signal?: AbortSignal,
): Promise<readonly string[]> => call.async('projectDirectories', folders, cwd, signal);
