import type { Settings } from '../../protocol/official.ts';
import type { PolicyView } from '../view.ts';
import type { ManagedComposition } from './compose.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The parent settings an embedding host passes through the Agent SDK's `managedSettings` option
 * (managed-settings.md, "Let an embedding host add policy"; claude-apps-gateway.md, "How hook and
 * skill keys in parent settings take effect"). One rule, applied where the layers are read
 * (`parentSettingsOf`) and again over any view a host built itself (`applyParentRules`):
 *
 * - "By default, Claude Code ignores parent settings whenever an admin source is present";
 *   `parentSettingsBehavior: "merge"` in the highest-priority managed source keeps them.
 * - The checks Claude Code applies "to parent-supplied values on their own": a value of the managed
 *   settings it applies blocks the parent's for the keys below.
 * - An empty parent allowlist is ignored when the administrator "defines `hooks` or enables a plugin".
 *
 * Which keys a parent may carry at all is the key table's (`KEY_SPECS`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/parent.rs`, docs/RUST.md).
 */
const call = coreModule('settings/managed/parent');

export interface ParentSettingsContext {
  /** An administrator deploys managed settings on this machine, whether or not the host passed them. */
  readonly hasManagedSettings: boolean;
  /** The administrator set `parentSettingsBehavior: "merge"`. */
  readonly parentMerge: boolean;
}

export interface ParentOutcome {
  /** The parent settings that count; `undefined` when an admin source is present and does not opt in. */
  readonly settings: Settings | undefined;
  /** The keys of the host dropped, by dotted path. */
  readonly dropped: readonly string[];
}

/** What a host's parent settings come to beside the managed tier the kit composed. */
export const parentSettingsOf = (
  composition: ManagedComposition,
  parent: Settings,
): ParentOutcome =>
  call(
    'parentSettingsOf',
    {
      layer: composition.layer,
      behavior: composition.behavior,
      hasManagedSettings: composition.hasManagedSettings,
      parentMerge: composition.parentMerge,
      admins: composition.admins,
    },
    parent,
  );

/**
 * The view every area reads: the parent layers under the same rule, over the managed layers of the
 * view. A view with nothing to drop is returned as it is, so applying the rule again costs nothing; a
 * kept parent layer keeps its `invalid` keys, whose fail-closed floors still apply.
 */
export const applyParentRules = (view: PolicyView, context: ParentSettingsContext): PolicyView =>
  call('applyParentRules', view, {
    hasManagedSettings: context.hasManagedSettings,
    parentMerge: context.parentMerge,
  });
