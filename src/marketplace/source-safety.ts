/**
 * The checks made before anything is fetched (plugins/marketplace-reference.md, "url plugin
 * source", "git-subdir plugin source", "npm plugin source", "archive plugin source"): pure, they
 * return issues, and a refusal "names the value and the reason". The docs give the rules and, for
 * npm, the categories; the wording of each message is the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/source_safety.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { GitRepository } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/source-safety');

export interface GitReading {
  readonly repo: GitRepository | undefined;
  readonly partialClone: boolean;
  readonly issues: readonly PluginIssue[];
}

/**
 * A git address in a `url` or `git-subdir` plugin source, or a `git` marketplace source. `url`
 * "doesn't take `owner/repo` shorthand", nor does a marketplace's `git`; `git-subdir` accepts it
 * as GitHub shorthand.
 */
export const readGitAddress = (text: string, shorthand: boolean, path: string): GitReading =>
  call('readGitAddress', text, shorthand, path);

/** GitHub naming rules for an owner and a repository ("Owner and repo must follow GitHub naming rules"). */
export const isGitHubRepository = (owner: string, repo: string): boolean =>
  call('isGitHubRepository', owner, repo);
