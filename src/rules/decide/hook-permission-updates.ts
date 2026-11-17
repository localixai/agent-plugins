import type { PermissionMode, PermissionUpdate } from '../../protocol/official.ts';
import type { DecideContext } from './context.ts';

/**
 * The permission update entries of a hook's answer as they take effect in this session (hooks.md,
 * "Permission update entries"): "`setMode` with `bypassPermissions` only takes effect if you
 * launched the session with bypass mode already available ... Otherwise the update is a no-op. The
 * update is also a no-op when `permissions.disableBypassPermissionsMode` disables the mode, or when
 * the session starts in restricted mode", and "`bypassPermissions` is never persisted as
 * `defaultMode` regardless of `destination`". `bypassAvailable` already folds in the launch, the
 * setting and `--restricted` (`startPermissions`). An entry that takes effect only in memory keeps
 * its place with `destination: "session"`.
 */

type Session = Pick<DecideContext, 'bypassAvailable'>;
type UpdateType = PermissionUpdate['type'];
type Members = { readonly [T in UpdateType]: Extract<PermissionUpdate, { readonly type: T }> };
type Registry = {
  readonly [T in UpdateType]: (
    update: Members[T],
    session: Session,
  ) => PermissionUpdate | undefined;
};

/** The modes the bypass rule is about. */
const IS_BYPASS: { readonly [M in PermissionMode]: boolean } = {
  default: false,
  acceptEdits: false,
  bypassPermissions: true,
  plan: false,
  dontAsk: false,
  auto: false,
};

const unchanged = (update: PermissionUpdate): PermissionUpdate => update;

const setMode = (update: Members['setMode'], session: Session): PermissionUpdate | undefined => {
  if (!IS_BYPASS[update.mode]) return update;
  return session.bypassAvailable ? { ...update, destination: 'session' } : undefined;
};

const IN_EFFECT: Registry = {
  addRules: unchanged,
  replaceRules: unchanged,
  removeRules: unchanged,
  setMode,
  addDirectories: unchanged,
  removeDirectories: unchanged,
};

function apply<T extends UpdateType>(
  registry: {
    readonly [P in T]: (update: Members[P], session: Session) => PermissionUpdate | undefined;
  },
  update: Members[T],
  session: Session,
): PermissionUpdate | undefined {
  return registry[update.type](update, session);
}

/** The entries to apply, in order: a `setMode` to bypass the session cannot take is left out. */
export function permissionUpdatesInEffect(
  updates: readonly PermissionUpdate[],
  session: Session,
): readonly PermissionUpdate[] {
  return updates.flatMap((update) => {
    const kept = apply(IN_EFFECT, update, session);
    return kept === undefined ? [] : [kept];
  });
}
