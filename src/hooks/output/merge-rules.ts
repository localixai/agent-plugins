import type { PluginIssue } from '../../format/issues.ts';
import type { HookEvent } from '../../protocol/official.ts';
import type { RegisteredHandler } from '../contracts.ts';
import type { MergedBlock } from '../verdict/types.ts';
import type { EventEffects } from './effects.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { CheckedOutput } from './schemas.ts';

/**
 * What the per-event merge rules share: one hook's output as the rules read it, and what a rule
 * returns. A rule gets every hook's read, in registry order (the order hooks are registered in, so
 * the result does not depend on which hook finishes first).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/merge_rules.rs`, docs/RUST.md).
 */

export interface Read {
  readonly handler: RegisteredHandler;
  /** The validated JSON output, `undefined` for a hook with none. */
  readonly output: CheckedOutput | undefined;
  /** `hookSpecificOutput` when the event honours it (the event's cap), else `undefined`. */
  readonly specific: object | undefined;
  /** The hook's plain stdout, as the verdict stage classified it. */
  readonly plainText:
    | { readonly text: string; readonly as: 'context' | 'compaction-instructions' }
    | undefined;
  /** A Monitor call reached the hook through a `Bash` matcher: its `allow` and `updatedInput` do not apply. */
  readonly effectsIgnored: boolean;
  /** The hook itself held the action back: by exit 2, a failure that blocks, or `decision: "block"`. */
  readonly blocks: boolean;
  /** Raw stdout of a `WorktreeCreate` command hook: its last non-empty line is the path. */
  readonly stdout: string | undefined;
}

export interface Merged {
  readonly effects: EventEffects;
  readonly issues: readonly PluginIssue[];
  /** A decision that holds the action back (a `deny`, a declined elicitation). */
  readonly block?: MergedBlock;
}

export type Merger = (reads: readonly Read[], facts: object) => Merged;

const call = coreModule('hooks/output/merge-rules');

export const NOTHING: Merged = call('NOTHING');

/** The last defined value: "choose last-in-registry-order wins" (spec/judgments.md). */
export const lastOf = <T>(values: readonly (T | undefined)[]): T | undefined =>
  call('lastOf', values);

export const eventSet = (...events: readonly HookEvent[]): ReadonlySet<HookEvent> =>
  call('eventSet', ...events);
