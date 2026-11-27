/**
 * `enabledPlugins`: whether a plugin is on and what decided it (settings-reference.md,
 * `enabledPlugins`; plugins/org.md, "Control matrix").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/plugins/enabled_plugins.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyScope } from '../layers.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/plugins/enabled-plugins');

/** Where an `enabledPlugins` decision came from: a file, or the plugin's own `defaultEnabled`. */
export type DecidedBy = PolicyScope | 'defaultEnabled';

export interface EnabledDecision {
  readonly on: boolean;
  readonly decidedBy: DecidedBy;
  /** A `false` in managed settings: "blocked from installation at every scope". */
  readonly blockedByManaged: boolean;
  /** The same `false`: "hidden from the marketplace". */
  readonly hiddenInMarketplace: boolean;
}

/**
 * Whether a plugin is on (settings-reference.md, `enabledPlugins`; plugins/org.md, "Control
 * matrix"). The highest file that has an entry for this id decides, so project beats user, local
 * beats project, and managed beats every other file: a managed `true` cannot be turned off and a
 * managed `false` cannot be turned on. With no entry anywhere the plugin follows its manifest
 * `defaultEnabled`, which is `true` when the manifest does not say (manifest-reference.md).
 *
 * A `--add-dir` directory ranks below user settings, and only its `true` counts: it never turns a
 * plugin off and never overrides another file (plugins/loading.md).
 *
 * Judgment (spec/judgments.md): the SDK also types a list or an object as an entry, which the
 * settings docs do not describe; only `false` turns a plugin off, so those read as enabled.
 */
export const enabledPlugin = (
  view: PolicyView,
  id: string,
  manifestDefaultEnabled = true,
): EnabledDecision => call('enabledPlugin', view, id, manifestDefaultEnabled);

/**
 * Whether managed settings force-enable the plugin. The id is matched whole, so a plugin of the
 * same name from another marketplace is not matched (settings-reference.md, `allowManagedHooksOnly`).
 */
export const forceEnabledByManaged = (view: PolicyView, id: string): boolean =>
  call('forceEnabledByManaged', view, id);
