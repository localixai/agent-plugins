/**
 * Where a settings object comes from, the layers in precedence order, and the layers that set a key.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/layers.rs`, docs/RUST.md).
 */
import type { Settings } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SettingsKey, ValueOf } from './general/keys.ts';

const call = coreModule('settings/layers');

/**
 * Where a settings object comes from, highest precedence first (settings.md, "Settings
 * precedence": managed, command line, project local, shared project, user). `flag` is the
 * `--settings` JSON. `parent` is the policy-tier settings an embedding host passes through the
 * Agent SDK's `managedSettings`; `addDir` is a `--add-dir` directory's `.claude/settings*.json`,
 * which carries two keys only. The docs rank `parent` nowhere (spec/judgments.md); they rank `addDir`
 * lowest of the sources of `enabledPlugins` (plugins/loading.md, "Find where a plugin is enabled").
 */
export const POLICY_SCOPES: readonly [
  'managed',
  'parent',
  'flag',
  'local',
  'project',
  'user',
  'addDir',
] = call('POLICY_SCOPES');
export type PolicyScope = (typeof POLICY_SCOPES)[number];

/** A higher number wins. */
export const SCOPE_RANK: { readonly [S in PolicyScope]: number } = call('SCOPE_RANK');

export interface PolicyLayer {
  readonly scope: PolicyScope;
  readonly settings: Settings;
  /**
   * The keys this source carried with a value Claude Code could not read and no value can stand for:
   * a repair removed them from `settings`, and the restrictive effect the docs give them is read
   * from here (managed-settings.md, "Keys that fail closed"; settings-reference.md,
   * `crossSessionInbound`). Absent when every key was readable.
   */
  readonly invalid?: readonly string[];
}

/** The layers, highest precedence first; two layers of one scope keep the order they were given. */
export const orderedLayers = (layers: readonly PolicyLayer[]): readonly PolicyLayer[] =>
  call('orderedLayers', layers);

/** One layer's value of a key, with the layer it came from. */
export interface KeyHolder<K extends SettingsKey> {
  readonly scope: PolicyScope;
  /** Position among the ordered layers: two files of one scope have two positions. */
  readonly position: number;
  readonly value: NonNullable<ValueOf<K>>;
}

/**
 * Every layer of `ordered` (highest precedence first, as `orderedLayers` gives them) that sets `key`
 * to a value its scope admits. A layer that does not carry the key (the key's scope line, or the
 * `--add-dir` rule) is skipped even when the layer was built by hand rather than read.
 */
export const holdersOf = <K extends SettingsKey>(
  ordered: readonly PolicyLayer[],
  key: K,
): readonly KeyHolder<K>[] => call('holdersOf', ordered, key);
