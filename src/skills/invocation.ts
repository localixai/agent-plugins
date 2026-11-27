/**
 * Who may invoke a skill and what the model and the `/` menu are told about it (skills.md,
 * "Control who invokes a skill" and "Override skill visibility from settings"). Pure: the host
 * reads the settings and passes `skillOverrides` in.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/invocation.rs`, docs/RUST.md).
 */
import type { InvocableHead } from '../format/skill-types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ModelListing, SkillOverrides } from './visibility.ts';

const call = coreModule('skills/invocation');

/**
 * A skill or a command file: both carry the two frontmatter fields (skills.md, "Choose where skills
 * load"). The body is not needed to decide who may invoke it or how it is listed, so a skill read
 * without its body (`InvocableHead`) is as good as a whole one.
 */
export type Invocable = InvocableHead;

/**
 * Who starts a skill: the user typing `/name`, Claude through the Skill tool, a subagent's `skills`
 * list (preloading), a session-scoped scheduled task that fires with the skill as its prompt.
 */
export const INVOKERS: readonly ['user', 'model', 'preload', 'scheduled-task'] = call('INVOKERS');
export type Invoker = (typeof INVOKERS)[number];

export interface InvocationPolicy {
  /** Typing `/name` runs the skill. */
  readonly user: boolean;
  /** Claude may load the skill on its own through the Skill tool. */
  readonly model: boolean;
  /** What the skill listing the model reads shows of it. */
  readonly listedToModel: ModelListing;
  /** The skill is a row of the `/` menu. */
  readonly inMenu: boolean;
  /** A subagent's `skills` field may preload it. */
  readonly preloadable: boolean;
  /** A scheduled task may fire with it as the prompt. */
  readonly scheduledPrompt: boolean;
}

/**
 * The whole matrix for one skill: the frontmatter and the override together. "Description not in
 * context" for `disable-model-invocation: true`; a skill the model cannot invoke is also not
 * preloadable and not run by a scheduled fire, because those "draw from the same set of skills
 * Claude can invoke" (sub-agents.md).
 */
export const invocationPolicy = (
  skill: Invocable,
  overrides: SkillOverrides = {},
): InvocationPolicy => call('invocationPolicy', skill, overrides);

export type DenialReason = 'disable-model-invocation' | 'user-invocable-false' | 'skill-override';

export type InvocationVerdict =
  | { readonly allowed: true }
  | { readonly allowed: false; readonly reason: DenialReason; readonly message: string };

export interface InvocationOptions {
  /**
   * The user's message names the skill after plain text (`go ahead and /deploy`, see
   * `readSkillMentions`): "The name counts as your permission for that message". Judgment
   * (spec/judgments.md): it lifts `disable-model-invocation` for the model's invocation in that
   * message only, never a `skillOverrides` state, and never a preload or a scheduled fire.
   */
  readonly permittedByMention?: boolean;
}

/** Whether `by` may start `skill`, with the reason and the text to give back when not. */
export const checkInvocation = (
  skill: Invocable,
  by: Invoker,
  overrides: SkillOverrides = {},
  options: InvocationOptions = {},
): InvocationVerdict => call('checkInvocation', skill, by, overrides, options);
