import type { Command, Skill } from '../format/skill-types.ts';
import type { Settings } from '../protocol/official.ts';
import type { Assert, Equal } from '../protocol/type-equality.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * The `skillOverrides` setting (skills.md, "Override skill visibility from settings"): each key is
 * a skill name and each value one of four states that decides who is told about the skill. Pure:
 * the host reads the settings and passes the map in. The invocation policy (`invocation.ts`), the
 * model listing and the `/` menu (`listing.ts`) all read the states through this one registry.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/visibility.rs`, docs/RUST.md).
 */
const call = coreModule('skills/visibility');

/** `skillOverrides` as the SDK types it: skill name to one of four states. */
export type SkillOverrides = NonNullable<Settings['skillOverrides']>;
export type OverrideState = SkillOverrides[string];

/** The four states, in the order the docs list them. */
export const OVERRIDE_STATES: readonly ['on', 'name-only', 'user-invocable-only', 'off'] =
  call('OVERRIDE_STATES');

/** @public A compile-time drift guard: a state Anthropic adds fails the typecheck here. */
export type OverrideStatesMatchSdk = Assert<Equal<(typeof OVERRIDE_STATES)[number], OverrideState>>;

/** What the model is told about a skill: name and description, name only, or nothing. */
export type ModelListing = 'description' | 'name' | 'hidden';

/** What one state leaves visible: "Listed to Claude" and "In `/` menu" of the docs table. */
export interface Exposure {
  readonly model: ModelListing;
  readonly user: boolean;
}

/** The docs table, a row per state: the registry the policy, the listing and the menu read. */
export const OVERRIDE_EXPOSURE: { readonly [S in OverrideState]: Exposure } =
  call('OVERRIDE_EXPOSURE');

type Overridable = Pick<Skill | Command, 'name' | 'origin'>;

/** What the core reads of a skill: its name and its origin's kind, so the rest never crosses. */
const facts = (
  skill: Overridable,
): { readonly name: string; readonly origin: { readonly kind: string } } => ({
  name: skill.name,
  origin: { kind: skill.origin.kind },
});

/** The state in force for a skill: absent from `skillOverrides` is `on`, and a plugin skill is never overridden. */
export const overrideState = (skill: Overridable, overrides: SkillOverrides = {}): OverrideState =>
  call('overrideState', facts(skill), overrides);

/** What the skill's state leaves visible to Claude and to the user. */
export const exposureOf = (skill: Overridable, overrides: SkillOverrides = {}): Exposure =>
  call('exposureOf', facts(skill), overrides);

/** What the `/skills` menu shows for a state: "The `/skills` menu labels the `user-invocable-only` state `user-only`". */
export const overrideMenuLabel = (state: OverrideState): string => call('overrideMenuLabel', state);
