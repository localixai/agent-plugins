import type { SlashCommand } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { Invocable } from './invocation.ts';
import type { ModelListing, SkillOverrides } from './visibility.ts';

/**
 * What the model and the `/` menu are told about the skills (skills.md, "Skill descriptions are cut
 * short", "Angle brackets in a skill description", "Override skill visibility from settings";
 * settings-reference.md, `skillListingMaxDescChars` and `skillListingBudgetFraction`). Pure: the host
 * passes the settings, the usage counts and the model's context window in.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/listing.rs`, docs/RUST.md).
 */

const call = coreModule('skills/listing');

/**
 * `SLASH_COMMAND_TOOL_CHAR_BUDGET`: "The budget scales dynamically at 1% of the context window, with a
 * fallback of 8,000 characters." The fallback applies when the host gives neither a window nor a count.
 */
export const FALLBACK_BUDGET_CHARS: number = call('FALLBACK_BUDGET_CHARS');

export interface ListingBudget {
  /** The model's context window in tokens. */
  readonly contextWindow?: number;
  /** `skillListingBudgetFraction`, greater than 0 and at most 1 (default 0.01). */
  readonly fraction?: number;
  /** A fixed character count (`SLASH_COMMAND_TOOL_CHAR_BUDGET`): it replaces the fraction of the window. */
  readonly chars?: number;
}

export interface ListingOptions {
  /** `skillOverrides`. */
  readonly overrides?: SkillOverrides;
  /** `skillListingMaxDescChars` (default 1536). */
  readonly maxDescChars?: number;
  /** With neither a window nor a character count the budget is 8,000 characters. */
  readonly budget?: ListingBudget;
  /** How often each skill (by command name) was invoked; absent counts as never. */
  readonly usage?: ReadonlyMap<string, number>;
  /**
   * The command names of skills whose `paths` have activated them in this session. Until then a
   * path-scoped skill is listed by name alone (the listing "always contains every skill name").
   */
  readonly activated?: ReadonlySet<string>;
}

export interface ListingEntry {
  readonly name: string;
  /** Absent for a name-only skill and for one whose description the budget dropped. */
  readonly description?: string;
}

/**
 * A command that is listed ahead of the skills with its description as written: a plugin's workflow
 * (oracle 2.1.319, spec/judgments.md). The description is cut and escaped as a skill's is.
 */
export interface LeadingCommand {
  readonly name: string;
  readonly description: string;
}

export interface ModelListingResult {
  readonly entries: readonly ListingEntry[];
  /** The skills that lost their description to the budget, in the order they lost it. */
  readonly descriptionsDropped: readonly string[];
  /** The characters the listing takes. */
  readonly chars: number;
  /** Still over the budget with every dropable description gone: names are never dropped. */
  readonly overBudget: boolean;
}

/** Judgment: the docs give the content of a line, not its form. */
export const listingLine = (entry: ListingEntry): string => call('listingLine', entry);

/** It sets `paths` and no touched file has activated it yet: the listing names it alone. */
export const isDormant = (skill: Invocable, activated?: ReadonlySet<string>): boolean =>
  call('isDormant', skill, activated);

/**
 * What the model's listing says of a skill before the budget: its description, its name alone (a
 * `name-only` override, a dormant `paths` skill), or nothing (`disable-model-invocation`, a hiding
 * override, no description: still invocable through the Skill tool, oracle 2.1.319).
 */
export const listedAs = (
  skill: Invocable,
  options: Pick<ListingOptions, 'overrides' | 'activated'> = {},
): ModelListing => call('listedAs', skill, options);

/**
 * The skills the model reads about, with the description of the least used dropped first until the
 * listing fits the budget. Among equally used skills the one listed last loses it first (judgment).
 */
export const modelListing = (
  skills: readonly Invocable[],
  options: ListingOptions = {},
  leading: readonly LeadingCommand[] = [],
): ModelListingResult => call('modelListing', skills, options, leading);

export interface MenuOptions {
  readonly overrides?: SkillOverrides;
}

/**
 * The rows of the `/` menu, in the official `SlashCommand` shape: the skills a user may type
 * (`user-invocable: false` and an `off` override are hidden). The row shows the description with
 * `<` and `>` escaped; judgment: not `when_to_use`, which the docs append only "in the skill
 * listing", and not capped. `argument-hint` is shown as written.
 */
export const menuEntries = (
  skills: readonly Invocable[],
  options: MenuOptions = {},
  leading: readonly LeadingCommand[] = [],
): SlashCommand[] => call('menuEntries', skills, options, leading);
