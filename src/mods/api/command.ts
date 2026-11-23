import { z } from 'zod';
import type { ModsNamespaceApi } from '../types.ts';
import { assertLoaded, call, parse, pluginOrigin, type Deps } from './call.ts';
import { commandRunEnd } from '../events/index.ts';
import { nameProblem } from './names.ts';
import { droppedCommand, refusing, waitsInQueue } from './refusal.ts';
import type { CommandListing } from './ports.ts';

/** The registration of `$.command.register` (api.md, "Add a command"). */
const registration = z.looseObject({
  name: z.string(),
  description: z.string().optional(),
  argumentHint: z.string().optional(),
  immediate: z.boolean().optional(),
});

/** What `$.command.run` takes: `{ command, args }`; `args` is the text typed after the command's name. */
const run = z.looseObject({ command: z.string(), args: z.string().default('') });

/**
 * Where a command's answer will show (claude-code/index.d.ts:1805, `CommandPresentation`): "The fact
 * `RenderViewport`'s `isFullscreen` carries on every terminal drawing, from the same source", and
 * the width; with nothing measured, the main screen's `false` and "80 where no terminal has measured".
 */
export interface CommandPresentation {
  readonly isFullscreen: boolean;
  readonly columns: number;
}

export const commandPresentation = (
  viewport: { readonly columns: number; readonly isFullscreen: boolean } | undefined,
): CommandPresentation => ({
  isFullscreen: viewport?.isFullscreen ?? false,
  columns: viewport?.columns ?? 80,
});

/** The refusal the docs quote for a taken name: `"/focus" refused: it is the built-in /focus`. */
const builtInRefusal = (name: string): string => `"/${name}" refused: it is the built-in /${name}`;

/** Resolves `{ command }`, the registered name (claude-code/index.d.ts:3288). */
function register(deps: Deps, event: unknown): { command: string } {
  const def = parse('$.command.register', registration, event);
  const problem = nameProblem('command', def.name);
  if (problem !== undefined) throw new Error(`$.command.register: ${problem}`);
  if (deps.ports.commands.builtIns().includes(def.name)) throw new Error(builtInRefusal(def.name));
  const taken = deps.registries.commands.get(def.name);
  // Judgment: the docs give the built-in case only; a name another mod has is refused the same way.
  if (taken && taken.plugin !== deps.plugin) {
    throw new Error(`"/${def.name}" refused: it is already registered by ${taken.plugin}`);
  }
  deps.registries.commands.set(def.name, { ...def, plugin: deps.plugin });
  return { command: def.name };
}

/**
 * `$.command.list()` (claude-code/index.d.ts:1776, `CommandInfo`): the commands mods registered, each
 * `source: 'plugin'` with the mod that registered it, after the host's own (built-in, user, MCP)
 * when its port lists them.
 */
async function listCommands(deps: Deps): Promise<readonly CommandListing[]> {
  const host = (await deps.ports.commands.list?.()) ?? [];
  const registered = deps.registries.commands
    .list()
    .map(({ name, description, argumentHint, plugin }) => ({
      name,
      description: description ?? '',
      source: 'plugin' as const,
      plugin,
      ...(argumentHint === undefined ? {} : { argumentHint }),
    }));
  return [...host, ...registered];
}

/**
 * A `command.run` answer as the model may read it (Agent Plugins Extended §10.30.1, "Hiding command
 * text from Claude", 2.1.326): `isTextDisplayOnly: true` stands only when a hook answered without
 * calling `next` (`answeredByHook`) for a command registered without `immediate`; otherwise it is
 * dropped, so the host gives the text to the model too. A host prints `text` and keeps it from the
 * model when the answer still carries the flag.
 */
export function displayOnlyReply(
  reply: unknown,
  facts: { readonly answeredByHook: boolean; readonly immediate: boolean },
): unknown {
  if (typeof reply !== 'object' || reply === null || !('isTextDisplayOnly' in reply)) return reply;
  if (facts.answeredByHook && !facts.immediate && reply.isTextDisplayOnly === true) return reply;
  const { isTextDisplayOnly: _dropped, ...rest } = reply;
  return rest;
}

/** The core of `$.command.run`: the host's command body, marked as running so a nested `runCommand` for it does not fire again. */
async function runCommandBody(deps: Deps, e: unknown): Promise<unknown> {
  const command = parse('$.command.run', run, e);
  const runBody = (queue: AbortSignal) => deps.ports.commands.run(command, queue);
  const body = () => waitsInQueue(deps.gate.interrupted(), runBody, droppedCommand(deps));
  return commandRunEnd(deps.registries.commands.get(command.command)?.plugin, command.command, () =>
    deps.registries.commandRuns.within(command.command, 'body', body),
  );
}

export function commandApi(deps: Deps): ModsNamespaceApi<'command'> {
  return {
    register: async (...args) => {
      assertLoaded(deps, 'command.register');
      return call(
        deps,
        'command.register',
        parse('$.command.register', registration, args[0]),
        (e) => Promise.resolve(register(deps, e)),
      );
    },
    run: async (...args) => {
      // `origin` and `presentation` the engine sets (claude-code/index.d.ts:1823, `CommandRunArgs`).
      const request = {
        ...parse('$.command.run', run, args[0]),
        origin: pluginOrigin(deps),
        presentation: deps.presentation?.() ?? commandPresentation(undefined),
      };
      let reached = false;
      const answer = await deps.replay.once(deps.plugin, 'command.run', () =>
        call(
          deps,
          'command.run',
          request,
          refusing(deps, 'command.run', (e) => ((reached = true), runCommandBody(deps, e))),
        ),
      );
      const immediate = deps.registries.commands.get(request.command)?.immediate === true;
      return displayOnlyReply(answer, { answeredByHook: !reached, immediate });
    },
    list: async () => call(deps, 'command.list', {}, () => listCommands(deps)),
  };
}
