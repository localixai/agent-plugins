import { coreModule } from '../wasm/bridge.ts';
import type { NamedInvocable, SkillLookup } from './lookup.ts';
import type { SkillMention } from './mention.ts';
import type { SkillOverrides } from './visibility.ts';

/**
 * Several skills in one message (skills.md, "Stack several skills in one message"): at the terminal
 * prompt `/write-tests /fix-issue 123` loads both skills and passes `123` to each. "Claude Code
 * expands the first skill plus up to five more stacked after it. Expansion stops at the first token
 * that isn't an inline user-invocable skill": a skill that runs as a forked subagent, or one whose
 * arguments may start with another command, ends it, and that token and everything after become the
 * argument text for every expanded skill. `readSkillMentions` finds the words; this decides which
 * of them are skills of the session that may be stacked.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/stack.rs`, docs/RUST.md).
 */

const call = coreModule('skills/stack');

export interface StackOptions {
  /** True at the terminal prompt (the default); through the Agent SDK or `claude -p` only the first skill loads. */
  readonly interactive?: boolean;
  readonly overrides?: SkillOverrides;
  /**
   * The commands whose arguments may start with another command (`/loop`): "expansion ends there".
   * The kit knows no bundled command, so the host names them.
   */
  readonly takesCommandArguments?: ReadonlySet<string>;
}

export interface Stack<E extends NamedInvocable> {
  /** The skills to expand, in the order typed: the first, then the ones stacked after it. */
  readonly skills: readonly E[];
  /** The text after the last expanded skill: passed as `$ARGUMENTS` to each of them. */
  readonly arguments: string;
  /** Every mention of the message as written: the direct and stacked ones, then the named ones. */
  readonly mentions: readonly SkillMention[];
}

/**
 * The skills a message starts with and the argument text they share. No skills when the message
 * does not start with a name of the catalog; the host then treats it as text.
 */
export const stackSkills = <E extends NamedInvocable>(
  message: string,
  catalog: SkillLookup<E>,
  options: StackOptions = {},
): Stack<E> => call('stackSkills', message, catalog, options);
