import type { GraphFacts } from '../marketplace/dependency-graph.ts';
import type { InstallStep } from '../marketplace/dependency-install-plan.ts';
import type { InstallScope } from '../marketplace/install-scopes.ts';
import type { RegisteredMarketplace } from '../format/marketplace/marketplace-names.ts';
import type { Sha256 } from '../ports/marketplace.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SavedValues } from './configure-plan.ts';
import type { ConfigOutcome, InstallOptions } from './install-config.ts';
import type { PendingInstall } from './install-start.ts';
import type { CommandOutput } from './result.ts';

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
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/install_plan.rs`, docs/RUST.md).
 */

const call = coreModule('cli/install-plan');

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

/** What `claude plugin install` prints and does, once the host added the marketplace and refreshed. */
export const planInstall = (
  pending: PendingInstall,
  found: InstallFound,
  sha256: Sha256,
): Promise<InstallRun> => call.async('planInstall', pending, found, sha256);
