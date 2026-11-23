import type { CatalogPlugin, GraphFacts } from '../marketplace/dependency-graph.ts';
import { installClosure, type InstallStep } from '../marketplace/dependency-install-plan.ts';
import type { InstallScope } from '../marketplace/install-scopes.ts';
import type { RegisteredMarketplace } from '../format/marketplace/marketplace-names.ts';
import type { Sha256 } from '../ports/marketplace.ts';
import { stopAtCommand } from './install-command-step.ts';
import type { SavedValues } from './configure-plan.ts';
import { applyConfig, type ConfigOutcome, type InstallOptions } from './install-config.ts';
import { closureRefusal, configRefusal, invalidEntryRefusal } from './install-refusals.ts';
import { invalidEntryMessage } from '../format/marketplace/entry-display.ts';
import {
  installEngineCheck,
  type InstallEngineCheck,
  type InstallEngineFacts,
} from './install-engines.ts';
import { successLines } from './install-success.ts';
import { recordsRefusal } from './records-refusal.ts';
import type { PendingInstall } from './install-start.ts';
import { beforeLines, locate, marketplaceRefusal } from './install-lookup.ts';
import { commandSucceeded, type CommandOutput, type ResultFields } from './result.ts';

/**
 * The second half of `claude plugin install <plugin>` (plugins/cli-reference.md, "plugin install",
 * "Accept a displayed install command"): the decision ladder once the host has added the
 * `--marketplace` source and refreshed. In order: the plugin is looked up (`Plugin "<name>" not found
 * in any configured marketplace` for a bare name nobody lists); one already installed at that scope
 * ends with `Plugin "<id>" is already installed (scope: <scope>)` and exit `0`, still planning any
 * missing dependency ("re-running `claude plugin install` on the dependent"); the `--config` values
 * are checked; the dependency closure is planned; a command-source or `headersHelper` entry prints its
 * command and needs `-y` or the digest (`decideAcceptance`). The refusals the docs do not quote are in
 * spec/judgments.md.
 */

/** What the host found after its `--marketplace` add and refresh. */
export interface InstallFound {
  /** The installed plugins and the marketplace catalogs, after the refresh. */
  readonly graph: GraphFacts;
  /** The catalog text the run read, after its own refresh: it goes into the digest `--accept-command` names. */
  readonly catalog: string;
  /** The plugin's `userConfig` and its bundled servers'. */
  readonly options: InstallOptions;
  /**
   * The marketplace `--marketplace` added or reused. `registered` is the fetch's fact for the
   * download-folder rule: "A new one is added under the same organization policy checks as `claude
   * plugin marketplace add`", so a name that command refuses refuses the install.
   */
  readonly addedMarketplace?: {
    readonly name: string;
    readonly alreadyKnown: boolean;
    readonly registered: readonly RegisteredMarketplace[];
    /** When `alreadyKnown`: the source it was added from; another source fails (§17.20.2). */
    readonly existingSource?: RegisteredMarketplace['source'];
  };
  /** The refresh the plan asked for failed: "the install proceeds from the cached catalog". */
  readonly refreshFailed?: boolean;
  /** The installed copy's directory exists; `false` for a record whose directory is gone ("not cached"), which a reinstall restores. */
  readonly installedCopyPresent?: boolean;
  /** The answer to `Run this command now? [y/N]`, once the host has asked. */
  readonly answer?: boolean;
  /**
   * The text `readInstalledPlugins` reported for a record of `installed_plugins.json` this version
   * cannot read, or a format it does not know; the command then refuses (§16.6.2 item 1).
   */
  readonly unreadableRecords?: string;
  /**
   * The `engines` of the version to install and the running client's version (spec §2.4): absent,
   * the range is not checked.
   */
  readonly engines?: InstallEngineFacts;
}

/** What the host carries out for a successful install. */
export interface InstallEffects {
  readonly id: string;
  readonly scope: InstallScope;
  /** Dependencies first; the plugin itself is installed after them unless it is already installed. */
  readonly steps: readonly InstallStep[];
  /** Install the plugin (not for one that was already installed). */
  readonly install: boolean;
  /**
   * Write `enabledPlugins[<id>]: true` at the scope: not for one already installed, nor for one
   * whose `defaultEnabled` is `false`, which "is installed but stays off until you turn it on"
   * (install.md, "Choose an install scope"). Every step is enabled whatever its own default.
   */
  readonly enable: boolean;
  readonly config: SavedValues;
  /** Values for each bundled server, by server name. */
  readonly servers: ConfigOutcome['servers'];
}

export type InstallRun =
  /** The host prints `before`, asks `prompt` and calls `planInstall` again with the answer. */
  | { readonly before: readonly string[]; readonly prompt: string }
  /** The plan `needs` catalog plugins fetched (`InstallPlan.needs`): the host fills them in `graph` and calls `planInstall` again. */
  | { readonly before: readonly string[]; readonly needs: readonly string[] }
  | {
      readonly before: readonly string[];
      readonly output: CommandOutput;
      readonly effects?: InstallEffects;
    };

const ended = (output: CommandOutput, before: readonly string[]): InstallRun => ({
  before,
  output,
});

const fieldsOf = (id: string, scope: InstallScope): ResultFields => ({ pluginId: id, scope });

function alreadyInstalled(
  id: string,
  pending: PendingInstall,
  found: InstallFound,
  before: readonly string[],
): InstallRun {
  const closure = installClosure(
    { root: id, scope: pending.scope, operation: 'reinstall' },
    found.graph,
  );
  if (closure.needs.length > 0) return { before, needs: closure.needs };
  const message = `Plugin "${id}" is already installed (scope: ${pending.scope})`;
  const output = commandSucceeded('install', message, { fields: fieldsOf(id, pending.scope) });
  if (!closure.ok || closure.steps.length === 0) return ended(output, before);
  return {
    before,
    output,
    effects: {
      id,
      scope: pending.scope,
      steps: closure.steps,
      install: false,
      enable: false,
      config: { settings: {}, secrets: {} },
      servers: {},
    },
  };
}

const isInstalledAt = (id: string, scope: InstallScope, graph: GraphFacts): boolean =>
  graph.installed.some((record) => record.id === id && record.scope === scope);

/** The entry's validity, the `engines` range and the `--config` values, checked before the closure. */
function checkedInput(
  { id, entry }: { readonly id: string; readonly entry: CatalogPlugin },
  pending: PendingInstall,
  found: InstallFound,
):
  | { readonly refusal: CommandOutput }
  | { readonly engine: InstallEngineCheck; readonly config: ConfigOutcome } {
  const invalid = invalidEntryMessage(entry);
  if (invalid !== undefined) return { refusal: invalidEntryRefusal(invalid) };
  const engine = installEngineCheck(id, found.engines);
  if (engine.refusal !== undefined) return { refusal: engine.refusal };
  const config = applyConfig(pending.given, found.options);
  return config.refused.length > 0 ? { refusal: configRefusal(config) } : { engine, config };
}

/** What `claude plugin install` prints and does, once the host added the marketplace and refreshed. */
export async function planInstall(
  pending: PendingInstall,
  found: InstallFound,
  sha256: Sha256,
): Promise<InstallRun> {
  const before = beforeLines(found);
  const records = recordsRefusal('install', pending.plugin, found.unreadableRecords);
  if (records !== undefined) return ended(records, before);
  const refused = marketplaceRefusal(pending, found);
  if (refused !== undefined) return ended(refused, []);
  const located = locate(pending, found);
  if ('end' in located) return ended(located.end, before);
  const { id, entry } = located;
  const { scope } = pending;
  if (isInstalledAt(id, scope, found.graph) && found.installedCopyPresent !== false) {
    return alreadyInstalled(id, pending, found, before);
  }
  const checked = checkedInput({ id, entry }, pending, found);
  if ('refusal' in checked) return ended(checked.refusal, before);
  const { engine, config } = checked;
  const closure = installClosure({ root: id, scope, operation: 'install' }, found.graph);
  if (closure.needs.length > 0) return { before, needs: closure.needs };
  if (!closure.ok) {
    return ended(
      closureRefusal(
        id,
        closure.issues.map((issue) => issue.message),
      ),
      before,
    );
  }
  const stopped = await stopAtCommand({ id, entry, before }, pending, found, sha256);
  if (stopped !== undefined) return stopped;
  const leftOut = closure.leftOut.map((one) => one.message);
  return {
    before,
    output: commandSucceeded(
      'install',
      successLines({
        id,
        scope,
        steps: closure.steps,
        config,
        options: found.options,
        leftOut,
        engineNote: engine.suffix,
      }),
      {
        fields: fieldsOf(id, scope),
      },
    ),
    effects: {
      id,
      scope,
      steps: closure.steps,
      install: true,
      enable: entry.defaultEnabled !== false,
      config: config.plugin,
      servers: config.servers,
    },
  };
}
