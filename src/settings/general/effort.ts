import type { EffortLevel } from '../../protocol/official.ts';
import type { SavedEffortLevel } from '../../protocol/effort.ts';
import type { PolicyLayer, PolicyScope } from '../layers.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The effort level a settings file saves for one model (settings-reference.md, `modelSettings` and
 * `effortLevel`): "A model's `effortLevel` here takes precedence over the top-level `effortLevel` in
 * the same settings file. Across files, Claude Code resolves each model separately: the
 * highest-precedence settings file that sets either an `effortLevel` for that model or a top-level
 * `effortLevel` that applies to that model decides". `modelSettings` is so no merged object: the
 * entry of a model in a lower file is never combined with a higher file's. A file saves only the
 * levels `SAVED_EFFORT_LEVELS` lists: `max` is valid for `maxEffortLevel` alone.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/effort.rs`, docs/RUST.md).
 */
const call = coreModule('settings/general/effort');

export interface ModelEffort {
  readonly level: SavedEffortLevel;
  readonly from: PolicyScope;
}

export interface ModelEffortFacts {
  /** The model's canonical name (`claude-opus-5-5`): the host maps the alias, dated, `[1m]` and provider spellings to it. */
  readonly model: string;
  /**
   * The top-level `effortLevel` of the user file applies to this model: in user settings it "keeps
   * applying where it applied before, on Opus 5, Fable 5.1, and earlier models"; in the other files
   * it applies to every model. Which models are "earlier" is the host's to say.
   */
  readonly userLevelApplies: boolean;
}

/** The level the files give a model, or `undefined` when none does. */
export const effortLevelFor = (
  layers: readonly PolicyLayer[],
  facts: ModelEffortFacts,
): ModelEffort | undefined =>
  call('effortLevelFor', layers, { model: facts.model, userLevelApplies: facts.userLevelApplies });

/**
 * The cap that applies to a model: "When several scopes set a cap, the lowest applies, so a cap set
 * in one scope can't be raised from another", and a file's `"max"` exempts the model from that
 * file's cap only, "Claude Code still applies caps from other sources". `undefined` when no file
 * caps the model (a `"max"` sets no cap). An organization's own effort limit for a model is not in
 * the files: the host takes the lower of the two with `capEffort`.
 */
export const effectiveCap = (
  layers: readonly PolicyLayer[],
  model: string,
): Exclude<EffortLevel, 'max'> | undefined => call('effectiveCap', layers, model);

/** The level a session runs at: `level`, or the cap when `level` is higher ("Any higher level runs at the cap instead"). */
export const capEffort = (level: EffortLevel, cap: EffortLevel | undefined): EffortLevel =>
  call('capEffort', level, cap);
