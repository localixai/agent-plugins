import type { PermissionBehavior } from '../../protocol/official.ts';
import type { PolicyScope } from '../layers.ts';

/**
 * Which settings files contribute permission rules and directories: `allowManagedPermissionRulesOnly`
 * (settings-reference.md), the policy an embedding host passes (the same entry; permissions.md,
 * "Settings precedence") and workspace trust (permissions.md, "Project allow rules and workspace
 * trust"). A rule is plain text with the file it came from; the host parses it with `parseRule`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/managed_only.rs`,
 * docs/RUST.md).
 */
export interface SettingsRule {
  readonly behavior: PermissionBehavior;
  readonly text: string;
  readonly scope: PolicyScope;
}

/** A `permissions.additionalDirectories` entry and the file it came from. */
export interface SettingsDirectory {
  readonly path: string;
  /** The highest file that lists it. */
  readonly scope: PolicyScope;
  /**
   * Only repository settings list it: `.claude/settings.json`, and `.claude/settings.local.json` when
   * git tracks it. Such a directory is a working directory for permissions but "doesn't count" for
   * `blockReadsOutsideWorkingDirectories` and `blockWritesOutsideWorkingDirectories`
   * (settings-reference.md).
   */
  readonly repositoryOnly: boolean;
}
