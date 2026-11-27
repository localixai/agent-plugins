/**
 * `claude plugin prune` as a plan (plugins/cli-reference.md, "plugin prune"): the orphaned
 * auto-installed dependencies at one scope (`user` by default), listed, asked about, or removed by
 * the flags and the terminal. The decision is `planPrune`'s (marketplace); this adds the scope, the
 * result a host prints and the answer to the prompt. "The exit code is `0` whatever you answer at
 * the prompt": a refusal without a terminal exits `0` too (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/prune_plan.rs`, docs/RUST.md).
 */
import type { GraphFacts } from '../marketplace/dependency-graph.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput } from './result.ts';
import type { ConfigScope } from './scope.ts';

const call = coreModule('cli/prune-plan');

export interface PruneArguments {
  readonly scope?: string;
  readonly dryRun?: boolean;
  readonly yes?: boolean;
}

export interface PruneFacts {
  readonly graph: GraphFacts;
  /** Both stdin and stdout are a terminal. */
  readonly tty: boolean;
}

/** The question the host asks, with what it removes if the answer is yes. */
export interface PruneQuestion {
  readonly prompt: string;
  readonly orphans: readonly string[];
}

export interface PrunePlan {
  readonly output: CommandOutput;
  /** The scope pruned. */
  readonly scope: ConfigScope;
  /** The plugin ids the host uninstalls from `scope`. */
  readonly remove: readonly string[];
  /** Set when the host asks first; `answerPrune` turns the answer into the final plan. */
  readonly ask?: PruneQuestion;
}

/** What `claude plugin prune` shows and removes, before any answer. */
export const planPruneCommand = (args: PruneArguments, facts: PruneFacts): PrunePlan =>
  call('planPruneCommand', args, facts);

/**
 * The plan once the user has answered `Remove? [y/N]`; a plan that asked nothing is returned as it
 * is: the same object, which a copy through the core would not be.
 */
export const answerPrune = (plan: PrunePlan, yes: boolean): PrunePlan =>
  plan.ask === undefined ? plan : call('answerPrune', plan, yes);
