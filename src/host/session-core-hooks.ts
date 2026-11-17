import type { CoreHookEvent, CoreHookHandler } from '../format/core/hooks.ts';
import { coreHandlerProcess } from '../format/core/subprocess.ts';
import type { PluginIssue } from '../format/issues.ts';
import type { HookPorts, HookSession } from '../hooks/index.ts';
import type { CoreSide } from '../hooks/core/combine.ts';
import { runCoreHandler } from '../hooks/core/run.ts';
import type { CoreOccurrence, CoreRun } from '../hooks/core/types.ts';
import { escapeReminderTags } from '../hooks/output/reminder-tags.ts';
import { limitText } from '../hooks/output/text-cap.ts';
import type { CorePackage } from '../loading/core/index.ts';
import { corePathsOf } from './core-servers.ts';
import { coreOf } from './core-view.ts';
import type { ActivePlugin, HostState } from './state.ts';

/**
 * The core side of one hook occurrence in a session (Agent Plugins Extended §18.1.2, core §7.3.3):
 * every handler of the event across the enabled core packages, in the order they loaded (J18.3),
 * each a fresh process with the core input on stdin. Core handlers are plugin hooks to the hook
 * policy (§18.1.2 item 5, J18.15): `disableAllHooks` turns them off, `allowManagedHooksOnly` keeps
 * only those of a package force-enabled in managed `enabledPlugins`, and a plugin-only lock leaves
 * them alone. Their environment is a Claude Code plugin hook's base (§3.3) with `CLAUDE_PROJECT_DIR`
 * and without `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA` or `CLAUDE_PLUGIN_OPTION_<KEY>` (J18.12),
 * then `PLUGIN_ROOT` and `PLUGIN_DATA` (core §9.1).
 */

export interface CoreHandlerUse {
  readonly state: HostState;
  readonly session: HookSession;
  readonly ports: HookPorts;
  /** The base environment of a plugin hook's process in this session (`subprocessEnvironment('hook')`). */
  readonly base: () => Readonly<Record<string, string>>;
}

interface Selected {
  readonly plugin: ActivePlugin;
  readonly core: CorePackage;
  readonly handler: CoreHookHandler;
  readonly index: number;
}

/** Whether the hook policy lets this package's handlers run (the gates of a plugin's hooks). */
function allowed(plugin: ActivePlugin, session: HookSession): boolean {
  const { disableAllHooks } = session;
  if (disableAllHooks.managedLocked || disableAllHooks.effective) return false;
  return (
    session.allowManagedHooksOnly !== true || session.forceEnabledPlugins?.has(plugin.id) === true
  );
}

/** The handlers that run on this event, in package load order. */
export function selectedCoreHandlers(
  use: CoreHandlerUse,
  event: CoreHookEvent,
): readonly Selected[] {
  return use.state.active.flatMap((plugin) => {
    const core = coreOf(plugin.plugin);
    if (core === undefined || !allowed(plugin, use.session)) return [];
    return (core.hooks[event] ?? []).map((handler, index) => ({ plugin, core, handler, index }));
  });
}

const CLAUDE_PLUGIN = /^CLAUDE_PLUGIN_(ROOT|DATA|OPTION_.*)$/;
const CLAUDE_PLUGIN_ANY_CASE = /^CLAUDE_PLUGIN_(ROOT|DATA|OPTION_.*)$/i;

/** The base of a core handler's process: no `CLAUDE_PLUGIN_*`, with `CLAUDE_PROJECT_DIR` (J18.12). */
function coreBase(use: CoreHandlerUse): Record<string, string> {
  const windows = use.state.options.windows ?? false;
  const scoped = windows ? CLAUDE_PLUGIN_ANY_CASE : CLAUDE_PLUGIN;
  const kept = Object.entries(use.base()).filter(([name]) => !scoped.test(name));
  return { ...Object.fromEntries(kept), CLAUDE_PROJECT_DIR: use.ports.locations.projectDir() };
}

async function invoke(
  use: CoreHandlerUse,
  selected: Selected,
  occurrence: CoreOccurrence,
  signal: AbortSignal,
): Promise<CoreRun> {
  const { plugin, handler, index } = selected;
  const paths = await corePathsOf(plugin, { ...use.ports, signal });
  const launch = coreHandlerProcess(handler, {
    paths,
    base: coreBase(use),
    platform: (use.state.options.windows ?? false) ? 'windows' : 'posix',
  });
  return runCoreHandler(
    use.ports.commands,
    {
      handlerId: `core:${plugin.id}:${occurrence.event}:${index}`,
      pluginId: plugin.id,
      event: occurrence.event,
      process: launch,
      label: [handler.command, ...(handler.args ?? [])].join(' '),
      input: occurrence.input,
    },
    signal,
  );
}

/** J18.5: a context string is capped, saved and escaped as a Claude Code hook's is (§7.5.3). */
async function contextsOf(
  runs: readonly CoreRun[],
  use: CoreHandlerUse,
  signal: AbortSignal,
): Promise<{ readonly contexts: readonly string[]; readonly issues: readonly PluginIssue[] }> {
  const limited = await Promise.all(
    runs.flatMap((run) =>
      run.result !== undefined && 'context' in run.result
        ? [
            limitText(
              `${run.handlerId}.additional_context`,
              run.result.context,
              use.ports.scratch,
              signal,
            ),
          ]
        : [],
    ),
  );
  return {
    contexts: limited.map((one) => escapeReminderTags(one.text)),
    issues: limited.flatMap((one) => one.issues),
  };
}

/**
 * Runs the selected handlers concurrently (core §7.3.3 lets the client) and waits for all of them,
 * a failure included, before the occurrence is acted on (core §7.3.5, §7.3.7).
 */
export async function runCoreSide(
  use: CoreHandlerUse,
  occurrence: CoreOccurrence,
  signal: AbortSignal,
): Promise<CoreSide> {
  const selected = selectedCoreHandlers(use, occurrence.event);
  const runs = await Promise.all(selected.map((one) => invoke(use, one, occurrence, signal)));
  const { contexts, issues } = await contextsOf(runs, use, signal);
  return { occurrence, runs, contexts, issues };
}
