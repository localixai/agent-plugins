import { installEngineCheck } from './install-engines.ts';
import { checkForUpdate, upToDateMessage } from '../marketplace/update-detection.ts';
import { UNKNOWN_VERSION } from '../marketplace/version-resolution.ts';
import type { InstallScope } from '../marketplace/install-scopes.ts';
import type { Sha256 } from '../ports/marketplace.ts';
import type { InstallStep } from '../marketplace/dependency-install-plan.ts';
import { DISABLE_NONESSENTIAL_TRAFFIC, isSet } from '../updates/env-flags.ts';
import {
  decideAcceptance,
  exclusiveAcceptFlags,
  type CommandAcceptance,
  type DisplayedCommand,
} from './command-acceptance.ts';
import { nameOf } from './plugin-ref.ts';
import {
  commandFailed,
  commandSucceeded,
  type CommandOutput,
  type ResultFields,
} from './result.ts';
import { recordsRefusal } from './records-refusal.ts';
import { chooseScope } from './scope.ts';
import { commandToAccept, failureLine, locate } from './update-locate.ts';
import type { UpdatableInstall, PluginUpdateArguments, PluginUpdateFacts } from './update-facts.ts';
import { dependencyList, updateDependencies } from './update-dependencies.ts';

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
 */

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
  /** The missing dependencies the update installs with it (§16.24.1). */
  readonly dependencies?: readonly InstallStep[];
}

export type UpdateRun =
  /** The host prints `before`, asks `prompt` and calls again with the answer. */
  | { readonly before: readonly string[]; readonly prompt: string }
  | { readonly before: readonly string[]; readonly retry: RetryRequest }
  /** The dependency plan `needs` catalog plugins fetched: the host fills them in `graph` and calls again. */
  | { readonly before: readonly string[]; readonly needs: readonly string[] }
  | {
      readonly before: readonly string[];
      readonly output: CommandOutput;
      readonly update?: UpdateEffects;
      /** The missing dependencies to install with the update, dependencies first (§16.24.1 item 4). */
      readonly dependencies?: readonly InstallStep[];
    };

const ended = (output: CommandOutput, before: readonly string[] = []): UpdateRun => ({
  before,
  output,
});

const checking = (argument: string): readonly string[] => [
  `Checking for updates for plugin "${argument}"…`,
];

const fieldsOf = (id: string, scope: InstallScope): ResultFields => ({ pluginId: id, scope });

/** The latest-version result, with the dependency-install retry when it is due. */
function atLatest(
  install: UpdatableInstall,
  args: PluginUpdateArguments,
  facts: PluginUpdateFacts,
): UpdateRun {
  const deps = updateDependencies(install, facts.graph, facts.env);
  if ('needs' in deps) return { before: checking(args.plugin), needs: deps.needs };
  const version = install.installedVersion ?? UNKNOWN_VERSION;
  const latest = withDependencies(upToDateMessage(nameOf(install.id), version), deps);
  const fields = fieldsOf(install.id, install.scope);
  const installing = stepsOf(deps.steps);
  const retries =
    install.dependencyInstall === 'unfinished' && !isSet(facts.env, DISABLE_NONESSENTIAL_TRAFFIC);
  if (!retries) {
    return {
      ...ended(commandSucceeded('update', latest, { fields }), checking(args.plugin)),
      ...installing,
    };
  }
  return {
    before: checking(args.plugin),
    retry: {
      id: install.id,
      scope: install.scope,
      argument: args.plugin,
      latest,
      fields,
      ...installing,
    },
  };
}

/** The success line with the dependencies listed, then the lines about the unresolved ones. */
const withDependencies = (
  line: string,
  deps: { readonly steps: readonly InstallStep[]; readonly lines: readonly string[] },
): string => [`${line}${dependencyList(deps.steps)}`, ...deps.lines].join('\n');

const stepsOf = (
  steps: readonly InstallStep[],
): { readonly dependencies?: readonly InstallStep[] } =>
  steps.length === 0 ? {} : { dependencies: steps };

type RetryOutcome = { readonly ok: true } | { readonly ok: false; readonly reason: string };

/** The result once the host has retried the dependency install: the failure fails the command, the success leaves the latest-version line. */
export function concludeRetry(request: RetryRequest, outcome: RetryOutcome): UpdateRun {
  const before = checking(request.argument);
  if (outcome.ok) {
    return {
      ...ended(commandSucceeded('update', request.latest, { fields: request.fields }), before),
      ...(request.dependencies === undefined ? {} : { dependencies: request.dependencies }),
    };
  }
  return ended(
    commandFailed('update', failureLine(request.argument, outcome.reason), {
      failureCode: 'dependency_install_failed',
      fields: request.fields,
    }),
    before,
  );
}

async function acceptanceOf(
  shown: DisplayedCommand,
  args: PluginUpdateArguments,
  facts: PluginUpdateFacts,
  sha256: Sha256,
): Promise<CommandAcceptance> {
  return decideAcceptance(
    {
      shown,
      catalog: facts.catalog,
      flags: {
        ...(args.yes === true ? { yes: true } : {}),
        ...(args.acceptCommand === undefined ? {} : { acceptCommand: args.acceptCommand }),
      },
      session: facts.session,
      tty: facts.tty,
      ...(facts.answer === undefined ? {} : { answer: facts.answer }),
    },
    sha256,
  );
}

/** The run that stops at the command: the question or the refusal; `undefined` when it runs. */
function stoppedAt(
  acceptance: CommandAcceptance,
  install: UpdatableInstall,
  shown: DisplayedCommand,
  argument: string,
): UpdateRun | undefined {
  if (acceptance.run) return undefined;
  const before = [...checking(argument), shown.command];
  if (acceptance.ask) return { before, prompt: acceptance.prompt };
  return ended(
    commandFailed('update', acceptance.line, {
      failureCode: 'command_not_accepted',
      fields: {
        ...fieldsOf(install.id, install.scope),
        shownCommand: { ...acceptance.shownCommand },
      },
    }),
    before,
  );
}

/** The install to update and the version it is updated to. */
interface UpdateTarget {
  readonly install: UpdatableInstall;
  readonly to: string;
}

async function updateTo(
  { install, to }: UpdateTarget,
  args: PluginUpdateArguments,
  facts: PluginUpdateFacts,
  sha256: Sha256,
): Promise<UpdateRun> {
  const shown = commandToAccept(install, facts.env);
  if (shown !== undefined) {
    const stopped = stoppedAt(
      await acceptanceOf(shown, args, facts, sha256),
      install,
      shown,
      args.plugin,
    );
    if (stopped !== undefined) return stopped;
  }
  const deps = updateDependencies(install, facts.graph, facts.env);
  if ('needs' in deps) return { before: checking(args.plugin), needs: deps.needs };
  const message = withDependencies(
    `Successfully updated plugin: ${nameOf(install.id)} to version ${to} (scope: ${install.scope})`,
    deps,
  );
  return {
    ...stepsOf(deps.steps),
    before: checking(args.plugin),
    output: commandSucceeded('update', message, { fields: fieldsOf(install.id, install.scope) }),
    update: {
      id: install.id,
      scope: install.scope,
      to,
      ...(install.installedVersion === undefined ? {} : { from: install.installedVersion }),
    },
  };
}

/** What `claude plugin update` prints and does. */
export async function planPluginUpdate(
  args: PluginUpdateArguments,
  facts: PluginUpdateFacts,
  sha256: Sha256,
): Promise<UpdateRun> {
  const chosen = chooseScope('update', args.scope);
  if ('reason' in chosen) return ended(chosen);
  const exclusive = exclusiveAcceptFlags({
    ...(args.yes === undefined ? {} : { yes: args.yes }),
    ...(args.acceptCommand === undefined ? {} : { acceptCommand: args.acceptCommand }),
  });
  if (exclusive !== undefined) return ended(exclusive);
  const records = recordsRefusal('update', args.plugin, facts.unreadableRecords);
  if (records !== undefined) return ended(records);
  const located = locate(args, facts, 'scope' in chosen ? chosen.scope : undefined);
  if ('end' in located) return ended(located.end);
  const { install } = located;
  const verdict = await checkForUpdate(install.subject, install.installedVersion, sha256);
  // A newer version outside its `engines` range is not one to move to (spec §2.4).
  const fits = installEngineCheck(install.id, install.engines).refusal === undefined;
  return verdict.check === 'new copy' && fits
    ? updateTo({ install, to: verdict.version }, args, facts, sha256)
    : atLatest(install, args, facts);
}
