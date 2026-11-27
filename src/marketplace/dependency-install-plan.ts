/**
 * The transitive install plan of a plugin's dependencies (plugins/install.md, "Plugins with
 * dependencies": "Claude Code also installs and enables the plugin's declared dependencies at the
 * same scope"; plugins/dependencies.md). Pure over `GraphFacts`: the plan says what to install, in
 * which order and at which version, or why nothing is installed; the host does it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/dependency_install_plan.rs`, docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { DependencyIssue } from './dependencies.ts';
import type { GraphFacts } from './dependency-graph.ts';
import type { ResolvedVersion } from './dependency-tags.ts';
import type { InstallScope } from './install-scopes.ts';

const call = coreModule('marketplace/dependency-install-plan');

/** Why Claude Code does not install a dependency itself. */
export type LeftOutReason = 'command-source' | 'headers-helper' | 'cross-marketplace';

/**
 * A dependency the install leaves out. A `command` or `headersHelper` one also refuses the install
 * until the user installs it by itself (§16.24); a cross-marketplace `plugin.json` one does not.
 */
export interface LeftOut {
  readonly id: string;
  readonly reason: LeftOutReason;
  readonly message: string;
  /** What the user runs first, when there is something. */
  readonly command?: string;
}

/**
 * The operations that install any missing declared dependency (plugins/dependencies.md, "Constrain
 * a dependency that has a non-git source"): `claude plugin install`, `/reload-plugins`, auto-update
 * of the dependent plugin's marketplace, re-running `claude plugin install` on the dependent, and
 * `claude plugin marketplace add`. The docs give the five one behaviour, so the plan does not depend
 * on which runs (judgment, spec/judgments.md); the plan only names it, for the host's report.
 */
export const DEPENDENCY_INSTALL_OPERATIONS: readonly [
  'install',
  'reload-plugins',
  'auto-update',
  'reinstall',
  'marketplace-add',
] = call('DEPENDENCY_INSTALL_OPERATIONS');
export type DependencyInstallOperation = (typeof DEPENDENCY_INSTALL_OPERATIONS)[number];

export interface InstallRequest {
  /** The plugin whose dependencies are installed, `name@marketplace`: the one being installed, or an installed one whose dependencies are missing. */
  readonly root: string;
  readonly scope: InstallScope;
  readonly operation: DependencyInstallOperation;
  /** The Claude Code version being emulated, for what only newer versions do (a local copy meeting an entry that names a marketplace, from 2.1.242); absent means a current one. */
  readonly hostVersion?: string;
}

/** `install`: not installed; `enable`: installed and disabled; `update`: installed at a version the combined ranges rule out. */
export type StepAction = 'install' | 'enable' | 'update';

export interface InstallStep {
  /** `name@marketplace`. */
  readonly id: string;
  readonly action: StepAction;
  /** The plugins that declare it, the ones in this plan and the root. */
  readonly requiredBy: readonly string[];
  /** Every range that constrains it: the ones of this plan's plugins and of the installed plugins. */
  readonly ranges: readonly string[];
  /** What the ranges resolved against; absent for `enable`, which fetches nothing. */
  readonly resolution?: ResolvedVersion;
  /** The version `claude plugin list` shows (a tag's, with its 12-character commit suffix). */
  readonly version?: string;
  /** Range problems the fetched copy will have when the plugin loads (a current copy outside a range): the dependent stays disabled. */
  readonly loadIssues: readonly DependencyIssue[];
}

export interface InstallPlan {
  readonly operation: DependencyInstallOperation;
  /** Dependencies before the plugins that need them. Empty when `issues` is not. */
  readonly steps: readonly InstallStep[];
  readonly leftOut: readonly LeftOut[];
  /** Why the install is refused whole; nothing is installed or changed then. */
  readonly issues: readonly PluginIssue[];
  /**
   * The catalog plugins (`name@marketplace`) the walk reached with `manifestUnread`, in walk order:
   * their `plugin.json` may declare more. The host fetches them, fills their facts and plans again;
   * the plan is final, and equal to the plan of a catalog read up front, once `needs` is empty.
   */
  readonly needs: readonly string[];
  /** The plan may be carried out: no issue and nothing needed. */
  readonly ok: boolean;
}

/**
 * The plan that installs, enables or updates the dependencies of `request.root`, transitively and
 * dependencies first. Per dependency: one already installed, enabled and inside every range is met
 * and needs no step, though its own declared dependencies are still planned; installed and disabled is enabled; installed outside the combined ranges is
 * updated; otherwise installed from the catalog. Each step carries its reason (`requiredBy`), the
 * ranges and what they resolve against (plugins/dependencies.md, "How a constraint resolves
 * against tags", "Combine constraints from several plugins"). A dependency in another marketplace
 * follows `crossMarketplaceDecision` with the root plugin's marketplace as the root: one declared in
 * a marketplace entry refuses the install, one in `plugin.json` is left out; a `command` source and
 * a `headersHelper` entry are left to the user and refuse the install until the user installs them
 * by itself (§16.24). A conflict between ranges, a range no tag in an own
 * repository meets, or a dependency its marketplace does not list refuses the whole plan:
 * `steps` is empty and nothing changes ("Plugin A and the dependency stay as they were").
 * Judgments (spec/judgments.md): the docs are silent on cycles and on depth, so each plugin is
 * planned once and the walk has no limit; a cross-marketplace dependency left out does not fail the install; a plugin
 * read from the catalog (the root above all) replaces its installed ranges with its new ones.
 * A catalog plugin with `manifestUnread` is planned from its entry's dependencies and listed in
 * `needs` (judgment `installNeeds`): the host fetches what the plan needs and plans again until
 * nothing is needed. Each round reads more facts and no fewer, and a plugin is needed once, so the
 * rounds end; a plan with `needs` is not final either way (a refusal may yet change).
 */
export const installClosure = (request: InstallRequest, facts: GraphFacts): InstallPlan =>
  call('installClosure', request, facts);
