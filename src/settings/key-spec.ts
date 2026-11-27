/**
 * One row per policy key: the files it may come from, how the files combine and how an invalid
 * value is repaired, from the key's entry in settings-reference.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/key_spec.rs`, docs/RUST.md).
 */
import type { Settings } from '../protocol/official.ts';
import type { PolicyKey } from '../protocol/settings-keys.ts';
import type { SettingsKey, ValueOf } from './general/keys.ts';
import type { PolicyScope } from './layers.ts';
import type { ValueSpec } from './value-spec.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('settings/key-spec');

/**
 * What to do with a value that does not have the type its key documents. One strategy per kind, so
 * what differs per key is a row of `KEY_SPECS`: the policy tier fails closed where the docs say so
 * (managed-settings.md, "Find entries Claude Code dropped"), every other source loses the value or
 * the entry.
 */
export type RepairKind =
  | 'boolean'
  | 'lockBoolean'
  | 'hooksSwitch'
  | 'string'
  | 'stringList'
  | 'closedStringList'
  | 'channelAllowlist'
  | 'allowedMcpServers'
  | 'deniedMcpServers'
  | 'strictSources'
  | 'blockedSources'
  | 'enabledPlugins'
  | 'marketplaceMap'
  | 'record'
  | 'skillOverrides'
  | 'workflowSize'
  | 'pluginOnly'
  | 'hooks'
  | 'asWritten'
  | 'providerAllowlist'
  | 'matchMode'
  | 'modelDenylist'
  | 'orgUuid'
  | 'gatewayNetworks'
  | 'permissions'
  | 'autoMode'
  | 'disableMode'
  | 'consentBoolean';

export interface KeySpec<K extends PolicyKey = PolicyKey> extends ValueSpec<Settings[K]> {
  readonly repair: RepairKind;
}

/**
 * The files a key may come from. The docs' scope names are `Any file`, `Managed`, `User or managed`
 * and `User, local, or managed` (plus `--settings`); the rest are the parent settings an embedding
 * host passes (agent-sdk/typescript.md, `managedSettings`), which only the keys the gateway page
 * names accept (claude-apps-gateway.md, "How hook and skill keys in parent settings take effect").
 * `userManagedFlag` is "User or managed" plus `--settings`, for the keys whose entry says so, and
 * `repositoryCannotElevate` admits any file except a repository's with an elevating value.
 */
export type ScopeRule =
  | 'any'
  | 'anyAndParent'
  | 'anyAndEmptyListFromParent'
  | 'managed'
  | 'managedAndParent'
  | 'user'
  | 'userOrManaged'
  | 'userManagedFlag'
  | 'userLocalManagedFlag'
  | 'repositoryCannotElevate';

/** Whether `key` read from `scope` with `value` is taken into account. */
export const admits = (key: SettingsKey, scope: PolicyScope, value: unknown): boolean =>
  call('admits', key, scope, value);

export const KEY_SPECS: { readonly [K in PolicyKey]: KeySpec<K> } = call('KEY_SPECS');

/** Every key the layers combine by name, with the row that says how. */
export const SPECS: { readonly [K in SettingsKey]: ValueSpec<ValueOf<K>> } = call('SPECS');
