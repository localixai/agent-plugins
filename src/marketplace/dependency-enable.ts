import type { PluginIssue } from '../format/issues.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { GraphFacts } from './dependency-graph.ts';
import type { InstallScope } from './install-scopes.ts';

/**
 * Enabling a plugin that declares dependencies (plugins/install.md, "Plugins with dependencies":
 * "Claude Code also enables the plugin's dependencies that are installed but disabled. If a declared
 * dependency isn't installed, the enable fails and the message tells you to install it first";
 * plugins/cli-reference.md, `plugin enable`: the three ways it fails). Pure: the host writes the
 * settings the plan names.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/dependency_enable.rs`, docs/RUST.md).
 */

const call = coreModule('marketplace/dependency-enable');

/** Why a dependency stops an enable. */
export type EnableBlock = 'not-installed' | 'policy-blocked' | 'disabled-at-higher-scope';

/** One dependency that stops the enable, with what the user can do about it. */
export interface EnableIssue extends PluginIssue {
  readonly block: EnableBlock;
  /** The dependency's id. */
  readonly dependency: string;
  /** `not-installed`: the `claude plugin install` command for it. */
  readonly command?: string;
  /** `disabled-at-higher-scope`: the scope that holds the `false`, where to enable it or to pass as `--scope`. */
  readonly scope?: InstallScope;
}

export interface EnablePlan {
  readonly ok: boolean;
  /** The dependencies to enable (installed but disabled), each after its own dependencies. */
  readonly enable: readonly string[];
  /** Every dependency that stops the enable; nothing is enabled when there is one. */
  readonly issues: readonly EnableIssue[];
}

/**
 * What enabling `id` at `target` does to its dependencies, transitively: the ones installed but
 * disabled are enabled, dependencies first; a dependency that is not installed, is blocked by the
 * plugin policy or is `false` at a scope that wins over `target` fails the enable (and so enables
 * nothing). Judgment: the cascade follows dependencies of dependencies, which the docs imply by
 * "the plugin's dependencies" being enabled so that the plugin loads; a dependency already enabled
 * by a winning scope is left alone, and one installed at any scope counts as installed.
 */
export const enablePlan = (id: string, target: InstallScope, facts: GraphFacts): EnablePlan =>
  call('enablePlan', id, target, facts);
