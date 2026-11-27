/**
 * The permission rules of a settings view as the rules module reads them, so a host that rebuilds
 * its permissions for each run (a new view, a new trust answer) needs no table of its own. Settings
 * and rules are one layer and never import each other (docs/ARCHITECTURE.md, "Layers"); this
 * joins them from above.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/host/permission_rules.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { PermissionRule } from '../rules/index.ts';
import type { PolicyView } from '../settings/index.ts';
import type { AddedDirectory, LocalFileFacts, ScopeTrust } from '../settings/general/index.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('host/permission-rules');

export interface PermissionRuleFacts {
  /** The user's trust decision for the folder of a repository file (`permissionRuleLists`). */
  readonly trusted: ScopeTrust;
  readonly local: LocalFileFacts;
  /** The `--settings` file as the host names it (a path, or the inline JSON's label). */
  readonly flagSettings: string;
}

export interface PermissionRulesOf {
  /** Every rule that applies, deny first, then ask, then allow; each with the source of its file. */
  readonly rules: readonly PermissionRule[];
  readonly directories: readonly AddedDirectory[];
  /** What the rules parser says about a rule it skips or ignores, each path led by the scope. */
  readonly issues: readonly PluginIssue[];
}

export const permissionRulesOf = (
  view: PolicyView,
  facts: PermissionRuleFacts,
): PermissionRulesOf => call('permissionRulesOf', view, facts);
