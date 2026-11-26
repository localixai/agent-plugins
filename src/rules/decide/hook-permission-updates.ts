/**
 * The permission update entries of a hook's answer as they take effect in this session (hooks.md,
 * "Permission update entries"): "`setMode` with `bypassPermissions` only takes effect if you
 * launched the session with bypass mode already available ... Otherwise the update is a no-op. The
 * update is also a no-op when `permissions.disableBypassPermissionsMode` disables the mode, or when
 * the session starts in restricted mode", and "`bypassPermissions` is never persisted as
 * `defaultMode` regardless of `destination`". `bypassAvailable` already folds in the launch, the
 * setting and `--restricted` (`startPermissions`). An entry that takes effect only in memory keeps
 * its place with `destination: "session"`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/decide/hook_permission_updates.rs`,
 * docs/RUST.md).
 */
import type { PermissionUpdate } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { DecideContext } from './context.ts';

const call = coreModule('rules/decide/hook-permission-updates');

type Session = Pick<DecideContext, 'bypassAvailable'>;

/** The entries to apply, in order: a `setMode` to bypass the session cannot take is left out. */
export const permissionUpdatesInEffect = (
  updates: readonly PermissionUpdate[],
  session: Session,
): readonly PermissionUpdate[] => call('permissionUpdatesInEffect', updates, session);
