/**
 * How the permission keys of several files combine. Every function takes the value of each file
 * that sets the key, highest precedence first (`KeyHolder` order), and returns one value.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/merge.rs`, docs/RUST.md).
 */
import type { Settings } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/permissions/merge');

/** The `permissions` object of the SDK's settings type, and the `autoMode` object. */
export type PermissionsBlock = NonNullable<Settings['permissions']>;
export type AutoModeBlock = NonNullable<Settings['autoMode']>;

/**
 * The `permissions` blocks of the files as one: `allow`, `ask`, `deny` and `additionalDirectories`
 * are the union of every file's entries; the modes and locks come from the highest file that sets
 * each, and so does every other field; `defaultMode` comes from the highest file that sets it (settings.md,
 * "Settings precedence"). Which file's `defaultMode` is taken into account is decided by
 * `startingMode`, which needs the scope this function does not see.
 */
export const mergePermissionBlocks = (blocks: readonly PermissionsBlock[]): PermissionsBlock =>
  call('mergePermissionBlocks', blocks);

/**
 * The `autoMode` blocks of the files as one: "When more than one of those files sets the same array,
 * Claude Code concatenates the entries". `classifyAllShell` is the highest file's here; its own row
 * (`autoMode.classifyAllShell`) holds the ladder that decides it.
 */
export const mergeAutoModeBlocks = (blocks: readonly AutoModeBlock[]): AutoModeBlock =>
  call('mergeAutoModeBlocks', blocks);
