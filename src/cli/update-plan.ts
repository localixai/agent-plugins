/**
 * `claude plugin update <plugin>` as a plan (plugins/cli-reference.md, "plugin update"): "Claude Code
 * prints `Checking for updates for plugin "<plugin>"…`, then the result. When nothing is newer, it
 * prints `<name> is already at the latest version (<version>).` and exits `0`, unless it retries the
 * plugin's dependency install and that install fails". Without `--scope` the plugin is updated "at the
 * most specific scope it's installed at ... local, project, user, then managed"; "You can pass a bare
 * plugin name ... When installed plugins from different marketplaces share the name, the command
 * refuses the update and lists the qualified commands"; "When the plugin is already at its latest
 * version, the command can also retry an unfinished dependency install in its cached copy" (plugins/
 * troubleshooting.md, "The packages it lists are not installed": `were not installed, because ...`
 * and `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` skip the retry); `-y` and `--accept-command` accept "a
 * changed install command from a command-source plugin" (`commandSourceRuns`, `decideAcceptance`).
 * What the docs leave open is in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/update_plan.rs`, docs/RUST.md).
 */
import type { InstallScope } from '../marketplace/install-scopes.ts';
import type { Sha256 } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput, ResultFields } from './result.ts';
import type { PluginUpdateArguments, PluginUpdateFacts } from './update-facts.ts';

const call = coreModule('cli/update-plan');

/** What the host carries out for an update that found a newer version. */
export interface UpdateEffects {
  readonly id: string;
  readonly scope: InstallScope;
  readonly from?: string;
  readonly to: string;
}

/** A retry of the dependency install in the cached copy: the host runs it and reports with `concludeRetry`. */
export interface RetryRequest {
  readonly id: string;
  readonly scope: InstallScope;
  readonly argument: string;
  readonly latest: string;
  readonly fields: ResultFields;
}

export type UpdateRun =
  /** The host prints `before`, asks `prompt` and calls again with the answer. */
  | { readonly before: readonly string[]; readonly prompt: string }
  | { readonly before: readonly string[]; readonly retry: RetryRequest }
  | {
      readonly before: readonly string[];
      readonly output: CommandOutput;
      readonly update?: UpdateEffects;
    };

type RetryOutcome = { readonly ok: true } | { readonly ok: false; readonly reason: string };

/** The result once the host has retried the dependency install: the failure fails the command, the success leaves the latest-version line. */
export const concludeRetry = (request: RetryRequest, outcome: RetryOutcome): UpdateRun =>
  call('concludeRetry', request, outcome);

/** What `claude plugin update` prints and does. The environment crosses as its own port. */
export const planPluginUpdate = (
  args: PluginUpdateArguments,
  facts: PluginUpdateFacts,
  sha256: Sha256,
): Promise<UpdateRun> => {
  const { env, ...rest } = facts;
  return call.async('planPluginUpdate', args, rest, env, sha256);
};
