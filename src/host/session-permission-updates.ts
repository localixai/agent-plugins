import type { HookOutcome } from '../hooks/index.ts';
import type { HookEvent } from '../protocol/official.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SessionOptions } from './session-types.ts';

/**
 * Implemented by the Rust core (`crates/agent-plugins/src/host/session_permission_updates.rs`,
 * docs/RUST.md).
 */
const call = coreModule('host/session-permission-updates');

/**
 * The outcome with the `updatedPermissions` of a `PermissionRequest` allow as they take effect
 * (hooks.md, "Permission update entries"): a `setMode` to `bypassPermissions` the session cannot
 * take is dropped, and one it can take stays in memory (`permissionUpdatesInEffect`).
 */
export const withUpdatesInEffect = <E extends HookEvent>(
  outcome: HookOutcome<E>,
  options: SessionOptions,
): HookOutcome<E> => call('withUpdatesInEffect', outcome, options);
