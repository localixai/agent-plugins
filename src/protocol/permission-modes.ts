/**
 * Implemented by the Rust core (`crates/agent-plugins/src/protocol/permission_modes.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { PermissionBehavior, PermissionMode } from './official.ts';
import type { Assert, Equal } from './type-equality.ts';

const call = coreModule('protocol/permission-modes');

/**
 * The permission modes of permissions.md ("Permission modes"), tied to the official
 * `PermissionMode` union: an SDK update that adds or drops a mode fails the typecheck here.
 */
export const PERMISSION_MODES: readonly [
  'default',
  'acceptEdits',
  'bypassPermissions',
  'plan',
  'dontAsk',
  'auto',
] = call('PERMISSION_MODES');

/** The three rule behaviours of permissions.md ("Manage permissions"): allow, ask and deny. */
export const PERMISSION_BEHAVIORS: readonly ['allow', 'deny', 'ask'] = call('PERMISSION_BEHAVIORS');

/** @public A compile-time drift guard: the typecheck fails when the SDK changes the union. */
export type PermissionModesMatchSdk = Assert<
  Equal<(typeof PERMISSION_MODES)[number], PermissionMode>
>;

/** @public A compile-time drift guard: the typecheck fails when the SDK changes the union. */
export type PermissionBehaviorsMatchSdk = Assert<
  Equal<(typeof PERMISSION_BEHAVIORS)[number], PermissionBehavior>
>;

/** A mode name as Claude Code accepts it: a mode itself or one of its aliases; else `undefined`. */
export const normalizePermissionMode = (raw: string): PermissionMode | undefined =>
  call('normalizePermissionMode', raw);
