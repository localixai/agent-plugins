/**
 * Whether a skill's `paths` activate it for a file (skills.md, frontmatter reference, `paths`):
 * "Patterns are relative to Claude's current working directory, and files outside that directory
 * don't match. Uses the same glob format as path-specific rules" (memory.md, "Path-specific
 * rules"): the gitignore syntax of the path rules, plus brace expansion.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/path_gate.rs`, docs/RUST.md).
 */
import type { MemoryRule } from '../format/memory-rule.ts';
import type { InvocableFrontmatter } from '../format/skill-types.ts';
import type { PathContext } from '../rules/types.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('skills/path-gate');

export interface PathGateOptions {
  /** The host's path flavour; Windows paths are matched in POSIX form (default `posix`). */
  readonly platform?: PathContext['platform'];
}

/**
 * Whether the skill is active for `filePath` when the session's directory is `cwd`. A skill with no
 * `paths` is always active; a file outside `cwd` (a sibling worktree, a parent) never matches.
 */
export const skillActivatesFor = (
  skill: Pick<InvocableFrontmatter, 'paths'>,
  filePath: string,
  cwd: string,
  options: PathGateOptions = {},
): boolean => call('skillActivatesFor', { paths: skill.paths }, filePath, cwd, options);

/**
 * The folder a rule's patterns are relative to (memory.md, "Path-specific rules"): for a project
 * rule "the directory that contains the rule's `.claude` folder" (the last `.claude/rules/` in its
 * path, so a nested folder's rule is relative to its own subdirectory); for a user rule, and a
 * project rule whose path names no such folder, the session's primary working directory `cwd`.
 */
export const ruleBase = (
  rule: Pick<MemoryRule, 'path' | 'scope'>,
  cwd: string,
  options: PathGateOptions = {},
): string => call('ruleBase', { path: rule.path, scope: rule.scope }, cwd, options);

/**
 * Whether a memory rule loads for `filePath`, which Claude read, wrote or edited: "A path-scoped
 * rule loads when Claude uses the Read, Write, or Edit tool on a matching file". A rule with no
 * `paths` is "loaded at launch", so it matches every file.
 */
export const ruleActivatesFor = (
  rule: Pick<MemoryRule, 'path' | 'scope' | 'paths'>,
  filePath: string,
  cwd: string,
  options: PathGateOptions = {},
): boolean =>
  call(
    'ruleActivatesFor',
    { path: rule.path, scope: rule.scope, paths: rule.paths },
    filePath,
    cwd,
    options,
  );
