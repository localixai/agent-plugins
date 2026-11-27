import type { ParentSettingsContext } from '../managed/parent.ts';
import type { PluginId } from '../plugins/plugin-id.ts';
import type { PolicyView } from '../view.ts';
import type { HttpHookUrlPolicy } from './http-policy.ts';
import type { PolicyHookSource, HookVerdict } from './origins.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/hooks/policy');

/**
 * The commands that follow the hooks (settings-reference.md, `disableAllHooks` "Status line and file
 * suggestion gates" and `allowManagedHooksOnly`; goal.md, "Requirements"): the status line, the file
 * suggestion command, the subagent status line, plugin monitors and `/goal`. The kit gives the
 * decision; rendering, starting a monitor and running a command stay the host's.
 */
export interface CommandGates {
  /**
   * "Off entirely": managed settings set `disableAllHooks`, or the folder is not trusted under the
   * workspace trust rule of hooks in settings files. Checked before the narrowing.
   */
  readonly commandsDisabled: {
    readonly statusLine: boolean;
    readonly fileSuggestion: boolean;
    readonly subagentStatusLine: boolean;
  };
  /**
   * "Narrowed to managed settings": `allowManagedHooksOnly`, a `disableAllHooks` that is `true`
   * outside managed settings after precedence, or `--safe-mode`. A managed value runs; any other is
   * skipped without a warning.
   */
  readonly commandSettingsNarrowedToManaged: boolean;
  /**
   * A plugin's monitors do not start under `allowManagedHooksOnly` unless managed settings
   * force-enable the plugin, matched on the whole `plugin@marketplace` id.
   */
  monitorsRun(plugin: PluginId): boolean;
  /** `/goal` "depends on hooks": not while hooks are disabled, restricted to managed ones, or untrusted. */
  readonly goalCommandAvailable: boolean;
}

/** The facts about the session the host knows and the settings do not. */
export interface CommandSessionFacts {
  /** The folder is trusted. Defaults to `true`. */
  readonly folderTrusted?: boolean;
  /** Claude Code was started with `--safe-mode`. Defaults to `false`. */
  readonly safeMode?: boolean;
}

/** What the host tells the hook policy besides the layers. */
export type HookPolicyContext = ParentSettingsContext & CommandSessionFacts;

/**
 * The keys that decide which hooks run, as one object over a view (docs/API.md, "Settings"): the
 * hooks engine asks `runs(origin)` before it runs any hook and `httpHook(url, allowedEnvVars)` before
 * it sends an HTTP request. The same decision as `hookSessionGates`, in the shape of questions.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/hooks/policy.rs`, docs/RUST.md).
 */
export interface HookPolicy extends CommandGates {
  runs(origin: PolicyHookSource): HookVerdict;
  httpHook: HttpHookUrlPolicy['check'];
}

/** The hook policy over a view the parent rules were already applied to (`resolvePolicy`). */
export const hookPolicyOver = (effective: PolicyView, facts: CommandSessionFacts): HookPolicy =>
  call('hookPolicyOver', effective, {
    folderTrusted: facts.folderTrusted,
    safeMode: facts.safeMode,
  });

export const createHookPolicy = (view: PolicyView, context: HookPolicyContext): HookPolicy =>
  call('createHookPolicy', view, {
    hasManagedSettings: context.hasManagedSettings,
    parentMerge: context.parentMerge,
    folderTrusted: context.folderTrusted,
    safeMode: context.safeMode,
  });
