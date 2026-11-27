/**
 * Permission update entries (hooks.md, "Permission update entries"): the array `updatedPermissions`
 * returns and `permission_suggestions` carries, six entry types, each with a `destination`. The
 * kit validates and normalizes them; writing them to a settings file is the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/permission_update.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../format/issues.ts';
import type { PermissionUpdate } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/permission-update');

export interface ParsedPermissionUpdates {
  readonly updates: readonly PermissionUpdate[];
  /** One error per entry that is not a valid permission update, `permissionUpdates[i].<field>`. */
  readonly issues: readonly PluginIssue[];
}

/** The valid entries of `raw` and an issue for each that is not: an entry never takes its neighbours down. */
export const parsePermissionUpdates = (raw: unknown): ParsedPermissionUpdates =>
  call('parsePermissionUpdates', raw);
