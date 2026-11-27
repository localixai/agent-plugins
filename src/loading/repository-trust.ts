/**
 * Whether a plugin that a repository supplies loads (plugins/loading.md, "Trust check and component
 * limits for repository plugins" and "Check why a repository plugin didn't load"). Pure: the host
 * answers the facts (`WorkspaceTrust` tells it whether the folder is trusted), the kit decides.
 *
 * A project-scope plugin "loads only after the same trust check that applies to project allow rules
 * in `.claude/settings.json`. Trusting a parent folder or running with `-p` isn't enough." A plugin
 * under the user's own `~/.claude/skills/` is exempt from the check.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/repository_trust.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { SkillsDirBlock } from './repository-types.ts';
import type { TrustSituation } from './trust-gates.ts';

const call = coreModule('loading/repository-trust');

/**
 * The places a repository-supplied (or user-skills-folder) plugin comes from: a `@skills-dir` plugin
 * of the project's `.claude/skills/`, a project-scope `@<marketplace>` plugin (enabled in the
 * repository's settings), and a `@skills-dir` plugin of the user's own `~/.claude/skills/`.
 */
export const REPOSITORY_ORIGINS: readonly [
  'skills-dir',
  'marketplace-project-scope',
  'user-skills-dir',
] = call('REPOSITORY_ORIGINS');
export type RepositoryOrigin = (typeof REPOSITORY_ORIGINS)[number];

/** What the host knows about the session and the folder. */
export interface RepositoryPluginFacts {
  readonly origin: RepositoryOrigin;
  /**
   * The folder's trust (`trustSituation`, `workspaceTrustState`): only `trusted`, the dialog accepted
   * for this exact folder, lets a checked plugin load; a parent folder's trust and a `claude -p` or SDK
   * run never do, and the messages say which of them it was.
   */
  readonly trust: TrustSituation;
  /** The session's primary working directory is the directory whose `.claude/skills/` holds the plugin. */
  readonly primaryDirIsRoot: boolean;
  /** The session runs in a worktree started from the desktop app. */
  readonly worktree: boolean;
  /** The restriction of managed settings that stops skills-directory plugins, when one applies. */
  readonly managedBlock?: SkillsDirBlock | undefined;
}

export const REPOSITORY_REASONS: readonly [
  'trusted',
  'user-level',
  'blocked-by-managed-settings',
  'not-primary-directory',
  'folder-not-trusted',
] = call('REPOSITORY_REASONS');
export type RepositoryReason = (typeof REPOSITORY_REASONS)[number];

/** Where the plugin's files are read from. */
export type RepositorySource = 'primary-directory' | 'main-checkout' | 'user-skills-directory';

export interface RepositoryDecision {
  readonly load: boolean;
  readonly reason: RepositoryReason;
  readonly from: RepositorySource;
}

/** The causes that apply to these facts, in the order of `CHECKS`. */
export const repositoryCauses = (facts: RepositoryPluginFacts): readonly RepositoryReason[] =>
  call('repositoryCauses', facts);

/** Whether a repository plugin loads, why not, and which checkout it is read from. */
export const decideRepositoryPlugin = (facts: RepositoryPluginFacts): RepositoryDecision =>
  call('decideRepositoryPlugin', facts);

/**
 * Whether the origin is "repository" content: the trust check and the component limits apply to it.
 * "Neither ... apply to a plugin under your own `~/.claude/skills/`."
 */
export const isRepositorySupplied = (origin: RepositoryOrigin): boolean =>
  call('isRepositorySupplied', origin);
