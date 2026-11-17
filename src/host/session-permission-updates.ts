import type { HookOutcome } from '../hooks/index.ts';
import type { HookEvent } from '../protocol/official.ts';
import { permissionUpdatesInEffect } from '../rules/index.ts';
import type { SessionOptions } from './session-types.ts';

/**
 * Whether bypass mode is available to this session: `permissionContext.bypassAvailable` when the
 * host gave it, otherwise only a session that is already in `bypassPermissions` (judgment,
 * spec/judgments.md: the session knows no more of its launch).
 */
const bypassAvailable = (options: SessionOptions): boolean =>
  options.permissionContext?.bypassAvailable ?? options.permissionMode === 'bypassPermissions';

/**
 * The outcome with the `updatedPermissions` of a `PermissionRequest` allow as they take effect
 * (hooks.md, "Permission update entries"): a `setMode` to `bypassPermissions` the session cannot
 * take is dropped, and one it can take stays in memory (`permissionUpdatesInEffect`).
 */
export function withUpdatesInEffect<E extends HookEvent>(
  outcome: HookOutcome<E>,
  options: SessionOptions,
): HookOutcome<E> {
  const answer = outcome.effects.permissionRequest;
  if (answer === undefined || !('updatedPermissions' in answer)) return outcome;
  if (answer.updatedPermissions === undefined) return outcome;
  const kept = permissionUpdatesInEffect(answer.updatedPermissions, {
    bypassAvailable: bypassAvailable(options),
  });
  const { updatedPermissions: _dropped, ...rest } = answer;
  const permissionRequest = kept.length === 0 ? rest : { ...rest, updatedPermissions: [...kept] };
  return { ...outcome, effects: { ...outcome.effects, permissionRequest } };
}
