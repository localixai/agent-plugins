import type { ValidationIssue } from '../../format/validate/finding.ts';
import type { PluginFiles } from '../../ports/index.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The Markdown components `claude plugin validate` reads (oracle 2.1.319): each skill folder's
 * `SKILL.md`, and every `.md` below `agents/` and `commands/` at any depth. Paths the manifest
 * declares are not read for frontmatter. A link is never followed: the folder or entry is skipped,
 * and a skills folder says so.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/validate/components.rs`,
 * docs/RUST.md).
 */
const call = coreModule('loading/validate/components');

/** The strings of a manifest key: one path, or the paths of a list. */
export const manifestPaths = (value: unknown): string[] => call('manifestPaths', value);

/** Link warnings and frontmatter findings of every skill, agent and command file, kind by kind. */
export const componentIssues = (
  files: PluginFiles,
  signal?: AbortSignal,
): Promise<ValidationIssue[]> => call.async('componentIssues', files, signal);
