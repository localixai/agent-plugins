import type { HooksSettings } from '../../protocol/hook-config.ts';
import type { PolicyScope } from '../layers.ts';
import type { ParentSettingsContext } from '../managed/parent.ts';
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/hooks/session-gates');

/**
 * What a hook session needs of the settings, as plain data: the one hook decision `createHookPolicy`
 * also gives, in the shape the hook engine's session takes (hooks.md, "Disable or remove hooks", "Hook
 * locations"). Settings is a lower layer than the engine, so the shape is structural, not a type
 * of `hooks`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/hooks/session_gates.rs`, docs/RUST.md).
 */
export interface HookSessionGates {
  /** `effective`: `disableAllHooks` left after precedence; `managedLocked`: set in managed settings. */
  readonly disableAllHooks: { readonly effective: boolean; readonly managedLocked: boolean };
  readonly allowManagedHooksOnly: boolean;
  /** `strictPluginOnlyCustomization` locks the `hooks` surface. */
  readonly pluginOnlyHooks: boolean;
  /** The merged HTTP hook allowlists; a key no file sets is absent (no limit). */
  readonly httpPolicy: {
    readonly allowedUrls?: readonly string[];
    readonly allowedEnvVars?: readonly string[];
  };
  /** The `hooks` of each settings file that declares any, highest precedence first. */
  readonly settingsHooks: readonly { readonly scope: PolicyScope; readonly hooks: HooksSettings }[];
  /** The plugin is force-enabled in managed `enabledPlugins`, which `allowManagedHooksOnly` exempts. */
  forceEnabled(pluginId: string): boolean;
}

/** The session facts of the hook gates over these settings, under the parent-settings rules. */
export const hookSessionGates = (
  view: PolicyView,
  context: ParentSettingsContext,
): HookSessionGates =>
  call('hookSessionGates', view, {
    hasManagedSettings: context.hasManagedSettings,
    parentMerge: context.parentMerge,
  });
