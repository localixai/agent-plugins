/**
 * What happens to a skill's content once it is invoked (skills.md, "Skill content lifecycle"): the
 * rendered `SKILL.md` "enters the conversation as a single message and stays there across later
 * turns", the file is not re-read, an `allowed-tools` grant clears at the next user message, a
 * re-invocation with identical rendered content adds a short note instead of a second copy, and
 * auto-compaction re-attaches the most recent invocation of each skill within a token budget. Pure:
 * the ledger is a value the host keeps next to its conversation.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/lifecycle.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('skills/lifecycle');

/** "keeping the first 5,000 tokens of each". */
export const REATTACH_TOKENS_PER_SKILL: 5000 = call('REATTACH_TOKENS_PER_SKILL');
/** "Re-attached skills share a combined budget of 25,000 tokens." */
export const REATTACH_TOKENS_TOTAL: 25000 = call('REATTACH_TOKENS_TOTAL');
/*
 * Judgment (spec/judgments.md): the docs count tokens and give no tokenizer, so the kit counts
 * by `estimateTokens` (format/token-estimate.ts); a host with a tokenizer passes its own counter.
 */

export interface TokenCounter {
  readonly count: (text: string) => number;
  /** The first `tokens` tokens of `text`. */
  readonly head: (text: string, tokens: number) => string;
}

/** One invocation that is in the conversation. */
export interface LoadedSkill {
  readonly command: string;
  /** The rendered content, as the model received it. */
  readonly content: string;
  /** Order of invocation, starting at 1: the most recent has the highest. */
  readonly order: number;
}

export interface SkillLedger {
  /** The copies of skill content in the conversation, in the order they entered (one skill may have several). */
  readonly loaded: readonly LoadedSkill[];
  /** The `allowed-tools` grants in force for the current turn only. */
  readonly grants: readonly string[];
  /** The `disallowed-tools` in force for the current turn only. */
  readonly disallowed: readonly string[];
}

export const EMPTY_LEDGER: SkillLedger = call('EMPTY_LEDGER');

export interface Invocation {
  readonly command: string;
  /** The rendered content: arguments and `` !`command` `` output already in it. */
  readonly content: string;
  readonly grants?: readonly string[];
  readonly disallowed?: readonly string[];
}

/** What enters the conversation for an invocation. */
export type SkillDelivery =
  | { readonly kind: 'content'; readonly text: string }
  | { readonly kind: 'already-loaded'; readonly text: string };

/**
 * Records an invocation. Identical rendered content to the copy already in context gives the note;
 * "When the rendered content differs, because the arguments changed or a dynamic context command
 * produced new output, Claude Code appends the full content again." The invocation re-applies its
 * grants and removals for this turn either way.
 */
export const invokeSkill = (
  ledger: SkillLedger,
  invocation: Invocation,
): { readonly ledger: SkillLedger; readonly delivery: SkillDelivery } =>
  call('invokeSkill', ledger, invocation);

/**
 * The user sends a message: the skills' content stays, but "an `allowed-tools` grant clears when you
 * send your next message", and so does the `disallowed-tools` restriction.
 */
export const startTurn = (ledger: SkillLedger): SkillLedger => call('startTurn', ledger);

export interface ReattachOptions {
  readonly perSkillTokens?: number;
  readonly totalTokens?: number;
  readonly tokens?: TokenCounter;
}

/**
 * What auto-compaction carries forward after the summary: the most recent invocation of each skill,
 * the first 5,000 tokens of each, filled into a 25,000-token budget "starting from the most recently
 * invoked skill, so older skills can be dropped entirely". Judgment (spec/judgments.md): a skill
 * that no longer fits is dropped whole, and so is every older one. Returned oldest first, as they
 * were invoked.
 */
export const reattachAfterCompaction = (
  ledger: SkillLedger,
  options: ReattachOptions = {},
): readonly LoadedSkill[] => call('reattachAfterCompaction', ledger, options);
