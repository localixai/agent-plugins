/** Implemented by the Rust core (`crates/agent-plugins/src/settings/repair/support.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { SettingsKey } from '../general/keys.ts';
import type { PolicyScope } from '../layers.ts';

const call = coreModule('settings/repair/support');

/**
 * What a repair knows about the value it reads. `closed` is true for the policy tier (managed and
 * parent settings), where an unreadable value of a restrictive key reads as its restrictive value
 * (managed-settings.md, "Keys that fail closed"); a user, project, local or `--settings` file only
 * loses the value or the entry (settings.md, "Fix a broken settings file").
 */
export interface RepairAt {
  readonly key: SettingsKey;
  readonly scope: PolicyScope;
  readonly closed: boolean;
}

/** The repaired value (`undefined`: the key does not apply) and what was said about it. */
export interface Repaired {
  readonly value: unknown;
  readonly issues: readonly PluginIssue[];
  /** The key was present and unreadable, and no value can stand for what that means: the layer records it (`PolicyLayer.invalid`). */
  readonly invalid?: boolean;
}

export type Repair = (value: unknown, at: RepairAt) => Repaired;

/** `<scope>:<field>`: the source and the field a report names (errors.md: `<source> (<field>)`). */
export const where = (at: RepairAt, field = ''): string => call('where', at, field);

export const kept = (value: unknown, issues: readonly PluginIssue[] = []): Repaired =>
  call('kept', value, issues);

/** A warning about the value, or about `field` inside it. */
export const note = (at: RepairAt, field: string, message: string): PluginIssue =>
  call('note', at, field, message);

/**
 * An invalid value that does not apply: dropped with a warning. The one wording of a key that is
 * dropped, the kit's own: errors.md quotes only `crossSessionInbound`'s ("This value was ignored").
 */
export const dropped = (at: RepairAt, reason: string): Repaired => call('dropped', at, reason);

/** `"<key>" must be <expected>; the key is ignored`. */
export const mustBe = (at: RepairAt, expected: string): Repaired => call('mustBe', at, expected);

/** `"<key>" was present but invalid; <effect>`: the report of a value that fails closed (managed-settings.md). */
export const presentButInvalid = (at: RepairAt, effect: string, field = ''): PluginIssue =>
  call('presentButInvalid', at, effect, field);

/** Splits a list into the entries `valid` accepts and a warning for each other one. */
export const stripInvalid = <T>(
  list: readonly unknown[],
  at: RepairAt,
  read: (entry: unknown) => T | undefined,
  reason: string,
): { entries: T[]; issues: PluginIssue[] } => call('stripInvalid', list, at, read, reason);
