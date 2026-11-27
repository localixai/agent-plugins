/**
 * The auto mode keys: the `autoMode` block that tells the classifier what the organization trusts,
 * and the three Booleans around it (settings-reference.md, `autoMode`, `autoMode.classifyAllShell`,
 * `useAutoModeDuringPlan`, `skipAutoPermissionPrompt`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/auto_mode.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/permissions/auto-mode');

/** The prose rule lists of the block, in the docs' order. */
export const AUTO_MODE_LISTS: readonly ['environment', 'allow', 'soft_deny', 'hard_deny'] =
  call('AUTO_MODE_LISTS');
export type AutoModeList = (typeof AUTO_MODE_LISTS)[number];

/** The auto mode settings in force. */
export interface AutoModeSettings {
  /** The classifier's prose lists, every file's entries joined, highest source first. `$defaults` is kept as written. */
  readonly lists: { readonly [L in AutoModeList]: readonly string[] };
  /** "Send every shell command through the classifier": a `true` from user, `--settings` or managed applies. */
  readonly classifyAllShell: boolean;
  /** Plan mode uses the classifier: on unless a `false` from managed, `--settings`, user or local turns it off. */
  readonly useAutoModeDuringPlan: boolean;
  /** The one-time notice of auto mode is skipped. */
  readonly skipAutoPermissionPrompt: boolean;
}

/** The auto mode settings of a view (docs/API.md, "Settings"). */
export const autoModeSettings = (view: PolicyView): AutoModeSettings =>
  call('autoModeSettings', view);
