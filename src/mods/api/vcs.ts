import { z } from 'zod';
import type { ModsNamespaceApi } from '../types.ts';
import { call, parse, type Deps } from './call.ts';
import type { ModVcs } from './ports.ts';

/**
 * `$.vcs` (experimental in the 2.1.330 declarations, claude-code/index.d.ts:4440; Agent Plugins
 * Extended §10.7.1, §10.19.8, J10.23): each method raises its `vcs.*` event with its argument, the
 * event the client also raises for its own version-control work, and resolves to that event's
 * result, which a hook answers in the client's place; the host's `vcs` port is the client's own
 * behaviour (Git, or nothing).
 */

const text = z.string();
const maybe = text.optional();

/** Each method's argument (`VcsDetectInput` and the others, :5552–:5787). */
const INPUTS = {
  detect: z.looseObject({ path: text }),
  diff: z.looseObject({
    path: text,
    vcs: text,
    base: z.enum(['default', 'head']),
    paths: z.array(text).optional(),
    isStatOnly: z.boolean().optional(),
  }),
  isolate: z.looseObject({
    for: z.enum(['agent', 'session']),
    from: text,
    name: text,
    base: z.enum(['fresh', 'head']),
    agentId: maybe,
    parentAgentId: maybe,
  }),
  list: z.looseObject({ path: text }),
  release: z.looseObject({
    path: text,
    vcs: text,
    name: text,
    isDiscard: z.boolean(),
    reason: text,
    agentId: maybe,
  }),
  status: z.looseObject({
    path: text,
    vcs: maybe,
    name: maybe,
    since: maybe,
    agentId: maybe,
  }),
} as const satisfies { readonly [M in keyof ModVcs]: z.ZodType };

type VcsMethod = keyof typeof INPUTS;

function method(deps: Deps, name: VcsMethod) {
  const label = `$.vcs.${name}`;
  const schema = INPUTS[name];
  return async (...args: readonly unknown[]): Promise<unknown> =>
    call(deps, `vcs.${name}`, parse(label, schema, args[0]), (e) =>
      deps.ports.vcs[name](parse(label, schema, e), deps.signal),
    );
}

export const vcsApi = (deps: Deps): ModsNamespaceApi<'vcs'> => ({
  detect: method(deps, 'detect'),
  diff: method(deps, 'diff'),
  isolate: method(deps, 'isolate'),
  list: method(deps, 'list'),
  release: method(deps, 'release'),
  status: method(deps, 'status'),
});
