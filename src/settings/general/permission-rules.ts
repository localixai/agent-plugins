/**
 * The permission rules every settings file adds up to, and the directories they add.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/permission_rules.rs`,
 * docs/RUST.md).
 */
import type { PolicyScope } from '../layers.ts';
import type { PolicyView } from '../view.ts';
import type { PermissionBehavior } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/general/permission-rules');

/** A rule as written in a settings file, with the file's scope. */
export interface RuleEntry {
  readonly rule: string;
  readonly scope: PolicyScope;
}

/** A directory `permissions.additionalDirectories` adds, with the file's scope. */
export interface AddedDirectory {
  readonly directory: string;
  readonly scope: PolicyScope;
}

/** The rules of each behaviour, highest scope first, a rule written in two files once, and the added directories. */
export type PermissionRuleLists = { readonly [B in PermissionBehavior]: readonly RuleEntry[] } & {
  readonly additionalDirectories: readonly AddedDirectory[];
};

/**
 * What the host learned about the local settings file. permissions.md ("When your local settings
 * file needs trust"): the file is normally your own, so "Claude Code applies its allow rules and
 * additional directories without the trust step. When the file is tracked in git, or `.claude` is a
 * symlink, Claude Code treats it as repository-supplied instead and holds its rules until you trust
 * the folder." Claude Code runs git only once the folder is trusted: before that the host has no
 * answer and says `tracked: true`, which holds the file like project settings.
 */
export interface LocalFileFacts {
  /** `.claude/settings.local.json` is tracked in git. */
  readonly tracked: boolean;
  /** `.claude` is a symlink. */
  readonly claudeDirIsSymlink: boolean;
}

/** Whether the user trusts the folder a file of this scope belongs to (the host asks the dialog's record). */
export type ScopeTrust = (scope: PolicyScope) => boolean;

/**
 * The permission rules every settings file adds up to (settings.md, "Lists merge instead of
 * overriding": `permissions.allow` combines across scopes; permissions.md, "Settings precedence": a
 * level cannot remove another's rule, so a managed deny stays whatever the others say). When the
 * managed tier sets `allowManagedPermissionRulesOnly` the other sources add none (settings-reference.md).
 * A repository's `allow` rules and `additionalDirectories` apply only once the folder is trusted
 * (the local file only when `local` says it is repository-supplied); `deny` and `ask` rules "only
 * restrict" and apply at once.
 */
export const permissionRuleLists = (
  view: PolicyView,
  trusted: ScopeTrust,
  local: LocalFileFacts,
): PermissionRuleLists => call('permissionRuleLists', view, trusted, local);
