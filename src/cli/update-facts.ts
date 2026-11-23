import type { InstallEngineFacts } from './install-engines.ts';
import type { GraphFacts } from '../marketplace/dependency-graph.ts';
import type { InstallScope } from '../marketplace/install-scopes.ts';
import type { UpdateSubject } from '../marketplace/update-detection.ts';
import type { Environment } from '../ports/environment.ts';
import type { CommandDeclaration } from '../updates/command-source-rerun.ts';

/** The arguments and facts of `claude plugin update` (docs/API.md, "CLI"). */

export interface PluginUpdateArguments {
  /** `name` or `name@marketplace`. */
  readonly plugin: string;
  readonly scope?: string;
  readonly yes?: boolean;
  readonly acceptCommand?: string;
  readonly json?: boolean;
}

/** The state of a cached copy's dependency install (`node_modules`): done, unfinished, or one that cannot run. */
export type DependencyInstallState = 'complete' | 'unfinished' | 'cannot-run';

/** A plugin installed at one scope, with what an update needs to know about it. */
export interface UpdatableInstall {
  /** `name@marketplace`. */
  readonly id: string;
  readonly scope: InstallScope;
  /** The version `installed_plugins.json` records. */
  readonly installedVersion?: string;
  readonly subject: UpdateSubject;
  /** For a `command` source: what its entry declares now and what the user accepted. */
  readonly command?: {
    readonly declared: CommandDeclaration;
    readonly accepted?: CommandDeclaration;
  };
  /** The entry sets a `headersHelper`: every update shows it. */
  readonly headersHelper?: string;
  readonly dependencyInstall: DependencyInstallState;
  /**
   * The `engines` of the newer version and the client's version (spec §2.4): a newer version whose
   * range excludes the client is not updated to. Absent, the range is not checked.
   */
  readonly engines?: InstallEngineFacts;
}

export interface PluginUpdateFacts {
  /** The plugins installed for the current project, at every scope. */
  readonly installs: readonly UpdatableInstall[];
  /** The marketplace catalog the run read, after any refresh: it goes into the digest `--accept-command` names. */
  readonly catalog: string;
  readonly env: Environment;
  readonly session: boolean;
  readonly tty: boolean;
  /** The answer to `Run this command now? [y/N]`, once the host has asked. */
  readonly answer?: boolean;
  /**
   * The text `readInstalledPlugins` reported for a record of `installed_plugins.json` this version
   * cannot read, or a format it does not know; the command then refuses (§16.6.2 item 1).
   */
  readonly unreadableRecords?: string;
  /**
   * The installed plugins and the catalogs after the refresh: the update installs the declared
   * dependencies the plugin is missing (§16.24.1 item 4). Absent, it plans none.
   */
  readonly graph?: GraphFacts;
}
