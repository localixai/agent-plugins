import type { PermissionMode } from '../../protocol/official.ts';
import type { PolicyView } from '../view.ts';

/**
 * permission-modes.md, "Eliminate prompts with auto mode": with `disableAutoMode: "disable"`, "A
 * session already running in auto mode leaves it when the setting reaches that session from an
 * admin-deployed source, and shows `auto mode disabled by settings`". The admin-deployed sources are
 * the managed ones (managed-settings.md, "Which managed source Claude Code uses"); the mode it
 * leaves for is Manual, as a start in auto under the setting is (judgment, spec/judgments.md).
 */

/** The notice the session shows, verbatim. */
export const AUTO_MODE_DISABLED_NOTICE = 'auto mode disabled by settings';

/** What a running session's mode becomes once new settings reach it. */
export interface RunningMode {
  readonly mode: PermissionMode;
  /** Set when the settings moved the session out of its mode. */
  readonly notice?: string;
}

const LEAVES: { readonly [M in PermissionMode]: boolean } = {
  default: false,
  acceptEdits: false,
  bypassPermissions: false,
  plan: false,
  dontAsk: false,
  auto: true,
};

/** Whether a managed source sets `disableAutoMode` (or `permissions.disableAutoMode`) to `"disable"`. */
function adminDisablesAutoMode(view: PolicyView): boolean {
  const top = view
    .eachFromHighest('disableAutoMode')
    .some((holder) => holder.scope === 'managed' && holder.value === 'disable');
  const nested = view
    .eachFromHighest('permissions')
    .some((holder) => holder.scope === 'managed' && holder.value.disableAutoMode === 'disable');
  return top || nested;
}

/** The mode of a session running in `current` once the settings of `view` reach it. */
export function runningModeOf(view: PolicyView, current: PermissionMode): RunningMode {
  return LEAVES[current] && adminDisablesAutoMode(view)
    ? { mode: 'default', notice: AUTO_MODE_DISABLED_NOTICE }
    : { mode: current };
}
