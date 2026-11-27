/**
 * A key whose managed value is not an override (settings.md, "Exceptions to managed settings
 * precedence"): a restrictive value from a lower scope stays in force. The rule is the same for
 * every key and only its data differs, so a key's row names a `ManagedException` and one function
 * applies it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/exceptions.rs`,
 * docs/RUST.md).
 */
import type { EffortLevel } from '../../protocol/official.ts';
import type { PolicyScope } from '../layers.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/general/exceptions');

export interface ManagedException<V> {
  /** The values the key reads, from the least restrictive to the most; a value off the ladder is not honored. */
  readonly ladder: readonly V[];
  /** The scopes whose more restrictive value beats the value that wins by precedence. */
  readonly honoredFrom: readonly PolicyScope[];
  /**
   * The scopes whose value never takes part in precedence and counts only when it is more
   * restrictive than the value that does (a repository's own files: the docs say a project or local
   * value "that isn't stricter is ignored").
   */
  readonly onlyWhenStricter: readonly PolicyScope[];
  /**
   * What a value of this key that no rung can stand for does when a source of `from` carries it: it
   * counts as `floor` whatever precedence says, so a higher source cannot undo it
   * (settings-reference.md, `crossSessionInbound`: an unrecognized value holds messages "even when a
   * source that takes precedence sets `accept`").
   */
  readonly unreadable?: { readonly floor: V; readonly from: readonly PolicyScope[] };
}

/** `maxEffortLevel`: "When several scopes set a cap, the lowest applies"; `"max"` sets no cap. */
export const LOWEST_CAP: ManagedException<EffortLevel> = call('LOWEST_CAP');

/**
 * The most restrictive of `values` on the ladder, `undefined` when none restricts more than the
 * first rung: the one "strict wins" step, for the view's rows and for the admin sources alike.
 */
export const strictestOf = <V>(ladder: readonly V[], values: readonly V[]): V | undefined =>
  call('strictestOf', ladder, values);
