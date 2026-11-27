import type { PluginIssue } from '../format/issues.ts';
import type { HookEvent } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { HookOutcome } from './outcome.ts';
import type { HandlerVerdict, MergedOutput, RunFacts } from './verdict/types.ts';

/**
 * What an event came to once every handler's run is read and the outputs merged (docs/API.md,
 * "Hooks"). A block of an exit code (exit 2, a failure) wins over a block of a JSON decision, in
 * whichever handler it sits: an exit-2 block's "fields ... can't override the block". Among exit
 * blocks the first in handler order wins; a JSON decision of that same hook speaks for it ("If your
 * hook also printed JSON that makes a blocking decision, Claude Code uses that decision's reason
 * instead"). Judgment: spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/assemble.rs`, docs/RUST.md).
 */

const call = coreModule('hooks/assemble');

/**
 * The facts of a verdict's run (`RunFacts`); a verdict the run stage did not time (one built by
 * hand) still names its handler and plugin, and says it finished.
 */
export const runFactsOfVerdict = (verdict: HandlerVerdict): RunFacts =>
  call('runFactsOfVerdict', verdict);

export const assemble = <E extends HookEvent>(
  event: E,
  verdicts: readonly HandlerVerdict[],
  merged: MergedOutput,
  issues: readonly PluginIssue[],
): HookOutcome<E> => call('assemble', event, verdicts, merged, issues);

/** The outcome of an event no handler ran for: nothing is held back. */
export const idleOutcome = <E extends HookEvent>(
  event: E,
  issues: readonly PluginIssue[],
): HookOutcome<E> => call('idleOutcome', event, issues);
