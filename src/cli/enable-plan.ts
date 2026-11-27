/**
 * `claude plugin enable <plugin>` as a plan (plugins/cli-reference.md, "plugin enable"; plugins/
 * install.md, "Plugins with dependencies"): which settings file gets `enabledPlugins[<id>]: true`
 * and which dependencies are enabled with it, or why nothing is. "When the plugin declares
 * dependencies, Claude Code enables them too. The command fails" when one is not installed (it
 * prints the `claude plugin install` command for each missing one), is blocked by the organization's
 * plugin policy, or "is set to `false` at a scope with higher precedence than the target scope". The
 * dependency walk is `enablePlan`'s.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/enable_plan.rs`, docs/RUST.md).
 */
import type { InstallScope } from '../marketplace/install-scopes.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput } from './result.ts';
import type { ToggleArguments, ToggleFacts } from './toggle-target.ts';

const call = coreModule('cli/enable-plan');

/** One `enabledPlugins` entry the host writes. */
export interface EnabledWrite {
  readonly scope: InstallScope;
  readonly id: string;
  readonly enabled: boolean;
}

export interface TogglePlan {
  readonly output: CommandOutput;
  /** The settings entries to write, dependencies first; empty when the command failed. */
  readonly writes: readonly EnabledWrite[];
}

/** What `claude plugin enable` writes and prints. */
export const planEnable = (args: ToggleArguments, facts: ToggleFacts): TogglePlan =>
  call('planEnable', args, facts);
