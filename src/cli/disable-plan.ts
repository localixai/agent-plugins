/**
 * `claude plugin disable [plugin]` as a plan (plugins/cli-reference.md, "plugin disable"): the
 * settings file that gets `enabledPlugins[<id>]: false`, or why nothing is written. "The command
 * fails for a plugin that is still required": another enabled plugin depends on it (the failure
 * "names the dependents to disable first", `disableDecision`), or the organization requires it as a
 * synced plugin ("saves nothing"). `--all` "Can't be combined with a plugin name or `--scope`"; with
 * neither a name nor `--all` the command prints `Please specify a plugin name or use --all to
 * disable all plugins` and exits `1`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/disable_plan.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { TogglePlan } from './enable-plan.ts';
import type { ToggleFacts } from './toggle-target.ts';

const call = coreModule('cli/disable-plan');

export interface DisableArguments {
  readonly plugin?: string;
  readonly scope?: string;
  readonly all?: boolean;
}

/** `Please specify a plugin name or use --all to disable all plugins`. */
export const NAME_OR_ALL: string = call('NAME_OR_ALL');

/** What `claude plugin disable` writes and prints. */
export const planDisable = (args: DisableArguments, facts: ToggleFacts): TogglePlan =>
  call('planDisable', args, facts);
