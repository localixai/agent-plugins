/**
 * Where a message names a skill (skills.md, "Control who invokes a skill", "Where you write the
 * skill's name"). A name at the start runs the skill directly; after plain text, "as a separate word
 * with no punctuation attached", it only counts as the user's permission for that message. The
 * reader finds the words; whether a skill of that name exists, and may run (`checkInvocation`), is
 * the host's next step.
 *
 * "Stack several skills in one message": at the terminal prompt `/write-tests /fix-issue 123` loads
 * both skills and passes `123` to each; the first plus up to five more are expanded, and expansion
 * stops at the first token that is not an inline user-invocable skill. Through `-p` or the Agent SDK
 * only the first skill loads, with the rest of the line as its arguments.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/mention.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('skills/mention');

export type SkillMention =
  | { readonly how: 'direct'; readonly command: string; readonly arguments: string }
  /** A skill stacked after the direct one: it gets the same trailing `arguments`. */
  | { readonly how: 'stacked'; readonly command: string; readonly arguments: string }
  | { readonly how: 'named'; readonly command: string };

/** A mention that starts the message: the skill runs. */
export type LeadingMention = Extract<SkillMention, { how: 'direct' | 'stacked' }>;

export const isLeading = (mention: SkillMention): mention is LeadingMention =>
  call('isLeading', mention);

export interface MentionOptions {
  /** True at the terminal prompt (the default); false through `-p` or the Agent SDK: only the first skill loads. */
  readonly interactive?: boolean;
  /**
   * Whether a command is an inline, user-invocable skill: a name that is not (a forked skill such as
   * `/code-review`, `/loop`, an unknown name) ends the stack and starts the arguments. Absent: every
   * name qualifies, so the host should pass it.
   */
  readonly stackable?: (command: string) => boolean;
  /**
   * The names the session lists, so a directory-qualified name (`/apps/web:deploy`, `/b/cli:verify`)
   * is one word although it holds `/`: a word that is exactly `/<a listed name>` is a mention.
   */
  readonly names?: ReadonlySet<string>;
}

export const readSkillMentions = (message: string, options: MentionOptions = {}): SkillMention[] =>
  call('readSkillMentions', message, options);
