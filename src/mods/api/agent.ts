import { z } from 'zod';
import type { ModsNamespaceApi } from '../types.ts';
import { assertLoaded, call, parse, type Deps } from './call.ts';
import { nameProblem } from './names.ts';
import { refusing } from './refusal.ts';
import { AGENT_STATUSES } from '../../protocol/mods-api-names.ts';
import { agentFullName } from './registries.ts';

/**
 * The registration of `$.agent.register`: `name` is the subagent type, the rest is the SDK's
 * `AgentDefinition`. Judgment: the docs name the method and no fields, so `name` mirrors
 * `$.command.register` and `$.tool.register`, and the definition keeps the SDK's two required fields.
 */
const registration = z.looseObject({
  name: z.string(),
  description: z.string(),
  prompt: z.string(),
});

const spawnRequest = z.looseObject({});

/**
 * `$.agent.call({ tool, input, agentId? }, { signal? })` (claude-code/index.d.ts:3922, v2.1.326): a
 * tool run as that agent's own call, `Workflow` refused (Agent Plugins Extended §10.7.1).
 */
const agentCall = z.looseObject({
  tool: z.string(),
  input: z.record(z.string(), z.unknown()),
  agentId: z.string().optional(),
});
const callOptions = z.looseObject({ signal: z.instanceof(AbortSignal).optional() });
/** Judgment (spec/judgments.md): the words of the refusal, which the declarations do not give. */
const WORKFLOW_REFUSED = "$.agent.call: Workflow cannot be called as an agent's own call";

async function callAs(deps: Deps, args: readonly unknown[]): Promise<unknown> {
  const request = parse('$.agent.call', agentCall, args[0]);
  const { signal } = parse('$.agent.call', callOptions, args[1] ?? {});
  if (request.tool === 'Workflow') throw new Error(WORKFLOW_REFUSED);
  const stop = signal === undefined ? deps.signal : AbortSignal.any([deps.signal, signal]);
  return call(deps, 'agent.call', request, (e) => {
    const asked = parse('$.agent.call', agentCall, e);
    if (asked.tool === 'Workflow') throw new Error(WORKFLOW_REFUSED);
    return deps.ports.agentCalls.call(asked, stop);
  });
}

/** A row of `$.agent.list()`: each has a `status` the reference lists (reference.md, "Mods API methods"). */
const listed = z.array(z.looseObject({ status: z.enum(AGENT_STATUSES) }));

/** Resolves `{ agent }`, the full name `<plugin>:<name>` (claude-code/index.d.ts:3394, :3402). */
function register(deps: Deps, event: unknown): { agent: string } {
  const { name, ...definition } = parse('$.agent.register', registration, event);
  const problem = nameProblem('subagent type', name);
  if (problem !== undefined) throw new Error(`$.agent.register: ${problem}`);
  deps.registries.agents.set(name, { plugin: deps.plugin, name, definition });
  return { agent: agentFullName(deps.plugin, name) };
}

export function agentApi(deps: Deps): ModsNamespaceApi<'agent'> {
  return {
    register: async (...args) => {
      assertLoaded(deps, 'agent.register');
      return call(deps, 'agent.register', parse('$.agent.register', registration, args[0]), (e) =>
        Promise.resolve(register(deps, e)),
      );
    },
    // A refused spawn resolves with `{ deny: reason }` and starts no subagent (api.md, "What refused calls return").
    spawn: async (...args) => {
      const request = parse('$.agent.spawn', spawnRequest, args[0]);
      return deps.replay.once(deps.plugin, 'agent.spawn', () =>
        call(
          deps,
          'agent.spawn',
          request,
          refusing(deps, 'agent.spawn', (e) =>
            deps.ports.agents.spawn(parse('$.agent.spawn', spawnRequest, e), deps.signal),
          ),
        ),
      );
    },
    call: async (...args) => callAs(deps, args),
    // `{ concurrent, depth, isBackgroundEnabled }` (`AgentLimits`, claude-code/index.d.ts:364).
    limits: async () => call(deps, 'agent.limits', {}, () => deps.ports.agentCalls.limits()),
    // Judgment: a row whose `status` the reference does not list is a host that drifted, so the call rejects.
    list: async () =>
      call(deps, 'agent.list', {}, async () =>
        parse('$.agent.list', listed, await deps.ports.agents.list()),
      ),
  };
}
