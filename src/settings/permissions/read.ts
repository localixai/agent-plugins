/**
 * Reading the `permissions` block of one settings source. Claude Code "repairs the `permissions`
 * ... blocks per field instead of dropping them whole" (managed-settings.md, "Keys that fail
 * closed"): in the policy tier a lock inside one reads as its restrictive value, an invalid
 * `defaultMode` reads as `default`, and while a `deny` or `ask` list cannot be read at all `allow`
 * and `additionalDirectories` are withheld. Any other source loses the value or the entry
 * (settings.md, "Fix a broken settings file").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/read.rs`, docs/RUST.md).
 */
import type { Repair, RepairAt, Repaired } from '../repair/support.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/permissions/read');

/** `"true"` or `"false"` as text reads as that Boolean in the policy tier, with a notice. */
export const flagValue = (
  raw: unknown,
  at: RepairAt,
  label: string,
  restrictive: boolean,
): Repaired => call('flagValue', raw, at, label, restrictive);

/** `permissions` of one source, repaired per field. */
export const permissionsBlock: Repair = (value, at) => call('permissionsBlock', value, at);

/** `disableAutoMode` at the top level of a file. */
export const disableValue: Repair = (value, at) => call('disableValue', value, at);
