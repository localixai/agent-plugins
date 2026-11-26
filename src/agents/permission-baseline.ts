import type { PermissionBehavior, PermissionMode } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/permission_baseline.rs`,
 * docs/RUST.md).
 */
const call = coreModule('agents/permission-baseline');

/**
 * The kinds of action the "Available modes" table tells apart. A read-only Bash command (the
 * built-in read-only set of permissions.md) is a `read`: permission-modes.md has Manual and
 * `dontAsk` still run it and `acceptEdits` prompt for the other commands "except the built-in
 * read-only set". Classifying a command as read-only is the host's.
 */
export const ACTION_CATEGORIES: readonly ['read', 'edit', 'filesystem-command', 'other'] =
  call('ACTION_CATEGORIES');
export type ActionCategory = (typeof ACTION_CATEGORIES)[number];

/** `classify`: the mode's background classifier decides (auto mode). */
export type BaselineDecision = PermissionBehavior | 'classify';

/**
 * One tool call as far as a mode's baseline is concerned. `inScope` is "paths in the working
 * directory or `additionalDirectories`": where it matters (`acceptEdits`, `dontAsk`, `auto`) the
 * table says so, elsewhere it is ignored. Deciding the scope of a path is the host's.
 */
export interface BaselineAction {
  readonly category: ActionCategory;
  readonly inScope: boolean;
}

/** What `mode` does with `action` before any rule, hook or classifier verdict. */
export const baselineDecision = (mode: PermissionMode, action: BaselineAction): BaselineDecision =>
  call('baselineDecision', mode, action);
