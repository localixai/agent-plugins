/**
 * The texts of repository plugins (plugins/loading.md, "Trust check and component limits for
 * repository plugins" and "Check why a repository plugin didn't load"). The docs quote no message for
 * a skipped monitor or bundle; the wording is the kit's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/repository_messages.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { RepositoryDecision, RepositoryPluginFacts } from './repository-trust.ts';

const call = coreModule('loading/repository-messages');

export const MONITORS_NOT_LOADED: string = call('MONITORS_NOT_LOADED');

export const BUNDLES_NOT_LOADED: string = call('BUNDLES_NOT_LOADED');

export const WORKTREE_NOTE: string = call('WORKTREE_NOTE');

export interface RepositoryExplanation {
  readonly decision: RepositoryDecision;
  /** Why it did not load: one sentence per cause that applies, in the decision's order. */
  readonly causes: readonly string[];
  /** What else is worth knowing: where the plugin was read from. */
  readonly notes: readonly string[];
}

/** The decision of `decideRepositoryPlugin` and, when the plugin did not load, every cause in words. */
export const explainRepositoryPlugin = (facts: RepositoryPluginFacts): RepositoryExplanation =>
  call('explainRepositoryPlugin', facts);
