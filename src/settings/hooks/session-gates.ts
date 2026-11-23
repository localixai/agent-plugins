import type { HooksSettings } from '../../protocol/hook-config.ts';
import type { PolicyScope } from '../layers.ts';
import type { PolicyView } from '../view.ts';
import { readSurfaceLocks } from '../customization/surfaces.ts';
import { applyParentRules, type ParentSettingsContext } from '../managed/parent.ts';
import { createHookGate, type HookGate } from './hook-gate.ts';
import {
  createHttpHookPolicy,
  httpHookLists,
  type HttpHookLists,
  type HttpHookUrlPolicy,
} from './http-policy.ts';

/**
 * What a hook session needs of the settings, as plain data: the one hook decision `createHookPolicy`
 * also gives, in the shape the hook engine's session takes (hooks.md, "Disable or remove hooks", "Hook
 * locations"). Settings is a lower layer than the engine, so the shape is structural, not a type
 * of `hooks`.
 */
export interface HookSessionGates {
  /** `effective`: `disableAllHooks` left after precedence; `managedLocked`: set in managed settings. */
  readonly disableAllHooks: { readonly effective: boolean; readonly managedLocked: boolean };
  readonly allowManagedHooksOnly: boolean;
  /** `strictPluginOnlyCustomization` locks the `hooks` surface. */
  readonly pluginOnlyHooks: boolean;
  /**
   * The merged HTTP hook allowlists that cover every hook but those of managed settings; a key no
   * file sets is absent (no limit).
   */
  readonly httpPolicy: HttpHookLists;
  /**
   * The same merged lists for the hooks of managed settings: a key is present only when managed
   * settings define it (spec §12.7.1, Judgment J12.18).
   */
  readonly managedHttpPolicy: HttpHookLists;
  /** The `hooks` of each settings file that declares any, highest precedence first. */
  readonly settingsHooks: readonly { readonly scope: PolicyScope; readonly hooks: HooksSettings }[];
  /** The plugin is force-enabled in managed `enabledPlugins`, which `allowManagedHooksOnly` exempts. */
  forceEnabled(pluginId: string): boolean;
}

/** The decision about hooks the settings make, read once from a view the parent rules were applied to. */
export interface HookDecision {
  readonly gate: HookGate;
  /** `strictPluginOnlyCustomization` locks the `hooks` surface. */
  readonly pluginOnlyHooks: boolean;
  readonly http: HttpHookUrlPolicy;
  readonly view: PolicyView;
}

export function hookDecision(effective: PolicyView): HookDecision {
  const pluginOnlyHooks = readSurfaceLocks(
    effective.get('strictPluginOnlyCustomization'),
  ).locked.has('hooks');
  return {
    gate: createHookGate(effective, { hooks: pluginOnlyHooks }),
    pluginOnlyHooks,
    http: createHttpHookPolicy(effective),
    view: effective,
  };
}

/** The decision as the hook engine's session takes it. */
function sessionGatesOf({ gate, pluginOnlyHooks, view }: HookDecision): HookSessionGates {
  return {
    disableAllHooks: {
      effective: gate.managedDisable || gate.otherDisable,
      managedLocked: gate.managedDisable,
    },
    allowManagedHooksOnly: gate.managedOnly,
    pluginOnlyHooks,
    httpPolicy: httpHookLists(view),
    managedHttpPolicy: httpHookLists(view, { managed: true }),
    settingsHooks: view
      .eachFromHighest('hooks')
      .map((holder) => ({ scope: holder.scope, hooks: holder.value })),
    forceEnabled: gate.forceEnabled,
  };
}

/** The session facts of the hook gates over these settings, under the parent-settings rules. */
export const hookSessionGates = (
  view: PolicyView,
  context: ParentSettingsContext,
): HookSessionGates => sessionGatesOf(hookDecision(applyParentRules(view, context)));
