/**
 * When the command of a `command` source runs again, and what follows a run (plugins/loading.md,
 * "When a command source re-runs"; plugins/host-marketplace.md, "Change the command of a command
 * source"). Pure: the host runs the command (`runCommandSource`); the kit says which runs happen,
 * which are skipped and why, what the user is asked, and what an Errors-tab entry says.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/updates/command_source_rerun.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { Environment } from '../ports/environment.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('updates/command-source-rerun');

/** What starts a decision: an explicit install or update, the start of a session, `/reload-plugins`. */
export const RERUN_EVENTS: readonly ['install', 'update', 'session-start', 'reload-plugins'] =
  call('RERUN_EVENTS');
export type RerunEvent = (typeof RERUN_EVENTS)[number];

/** The runs the docs list: "Every time you install or update", "Once per session", "installed version is missing". */
export const RUN_KINDS: readonly ['install', 'update', 'once-per-session', 'missing-version'] =
  call('RUN_KINDS');
export type RunKind = (typeof RUN_KINDS)[number];

/** The command and `mode` a user accepted, or that the catalog declares now. */
export interface CommandDeclaration {
  readonly command: string;
  readonly mode: 'copy' | 'link';
}

export interface CommandPlugin {
  /** `name@marketplace`. */
  readonly id: string;
  readonly enabled: boolean;
  /** The installed version's directory exists in the plugin cache. */
  readonly installedVersionInCache: boolean;
  /** What the marketplace entry declares now. */
  readonly declared: CommandDeclaration;
  /** What the user accepted when they installed or last updated; none for a plugin never installed. */
  readonly accepted?: CommandDeclaration;
}

export type RunSkip = 'command-not-accepted' | 'nonessential-traffic-disabled';

/** "Claude Code shows them the new command and asks them to accept it." */
export type Acceptance = { readonly ask: false } | { readonly ask: true; readonly showing: string };

/** "An entry in the `/plugin` Errors tab: the entry shows the new command and the `claude plugin update` command to run." */
export interface ErrorsTabEntry {
  readonly plugin: string;
  readonly command: string;
  readonly mode: CommandDeclaration['mode'];
  readonly updateCommand: string;
  readonly issue: PluginIssue;
}

export interface RerunPlan {
  /** The runs that start now. */
  readonly runs: readonly RunKind[];
  /** The runs held until the user accepts the command shown by `acceptance`: "has to accept the new command before Claude Code runs it". */
  readonly runsAfterAcceptance: readonly RunKind[];
  readonly skipped: readonly { readonly run: RunKind; readonly reason: RunSkip }[];
  /** Set for an install or update: whether the user must accept the command first. */
  readonly acceptance?: Acceptance;
  readonly errorsTab?: ErrorsTabEntry;
}

/**
 * The runs `event` brings about for one command-sourced plugin. `DISABLE_AUTOUPDATER` and the
 * marketplace's `autoUpdate` have no part in it: only the traffic variable and an unaccepted
 * command stop the background runs, and never an install or update.
 */
export const commandSourceRuns = (
  event: RerunEvent,
  plugin: CommandPlugin,
  env: Environment,
): RerunPlan => call('commandSourceRuns', event, plugin, env);

/** The runs that go ahead once the user has answered the question of `plan.acceptance`: a declined command never runs. */
export const runsAfterAnswer = (plan: RerunPlan, accepted: boolean): readonly RunKind[] =>
  call('runsAfterAnswer', plan, accepted);

/** What a finished run leaves to do with the version it produced. */
export interface RunResult {
  /** The version `installed_plugins.json` records; none for a plugin never installed. */
  readonly installedVersion: string | undefined;
  /** The version the hash of the command's output gives (`resolvePluginVersion`). */
  readonly computedVersion: string;
  /** A running interactive session exists to reload into. */
  readonly interactive: boolean;
  /** Reloading in place "would invalidate the session's prompt cache". */
  readonly reloadInvalidatesPromptCache: boolean;
}

export type AfterRun =
  | { readonly install: false; readonly followUp: 'nothing' }
  | {
      readonly install: true;
      readonly followUp: 'reload';
      /** "You see a notification that the plugin was reloaded." The reload switches what `/reload-plugins` switches. */
      readonly notice: 'plugin-reloaded';
    }
  | { readonly install: true; readonly followUp: 'ask-reload-plugins' | 'next-launch' };

/**
 * "When the command's hashed output has changed, Claude Code installs the result as a new version
 * and reloads it in the running interactive session"; if that "would invalidate the session's
 * prompt cache" it prompts for `/reload-plugins` instead. A session with nothing to reload into
 * loads the new version at its next launch (judgment, spec/judgments.md).
 */
export const afterCommandRun = (result: RunResult): AfterRun => call('afterCommandRun', result);
