import { isRecord } from '../../protocol/is-record.ts';
import { sameData } from '../events/spec.ts';

/**
 * What a plugin ships, by kind, as `plugin.register` hands it to a policy mod in `e.provides`
 * (Agent Plugins Extended §10.19.5 item 2; `PluginRegisterInput.provides`, claude-code/index.d.ts
 * of 2.1.330): a hook passes `next` the event with entries removed and those parts stay out while
 * the rest load. The client refuses a list that is not a subset, by value, of the one received, and
 * a narrowed `themes` or `bin`; a `provides` left out is restored.
 */
export const PROVIDES_KINDS = [
  'agents',
  'bin',
  'commands',
  'hooks',
  'lspServers',
  'mcpServers',
  'modules',
  'monitors',
  'outputStyles',
  'skills',
  'themes',
  'workflows',
] as const;
export type ProvidesKind = (typeof PROVIDES_KINDS)[number];

/** Each kind's entries: names (`skills`, `commands`) or objects (`modules`, `mcpServers`) as the host lists them. */
export type PluginProvides = { readonly [K in ProvidesKind]: readonly unknown[] };

/** The kinds a hook may not narrow: "and `themes` or `bin` narrowed" is refused (§10.19.5 item 2). */
const FIXED: { readonly [K in ProvidesKind]: boolean } = {
  agents: false,
  bin: true,
  commands: false,
  hooks: false,
  lspServers: false,
  mcpServers: false,
  modules: false,
  monitors: false,
  outputStyles: false,
  skills: false,
  themes: true,
  workflows: false,
};

/** A `provides` with every kind, the given ones filled in. */
export function fullProvides(given: Partial<PluginProvides> = {}): PluginProvides {
  const at = (kind: ProvidesKind): readonly unknown[] => given[kind] ?? [];
  return {
    agents: at('agents'),
    bin: at('bin'),
    commands: at('commands'),
    hooks: at('hooks'),
    lspServers: at('lspServers'),
    mcpServers: at('mcpServers'),
    modules: at('modules'),
    monitors: at('monitors'),
    outputStyles: at('outputStyles'),
    skills: at('skills'),
    themes: at('themes'),
    workflows: at('workflows'),
  };
}

const listOf = (value: unknown, kind: ProvidesKind): readonly unknown[] | undefined => {
  if (!isRecord(value)) return undefined;
  const list = value[kind];
  return Array.isArray(list) ? list : undefined;
};

/** Whether each entry of `passed` is one of `received`, by value, each used once at most. */
function isSubset(received: readonly unknown[], passed: readonly unknown[]): boolean {
  const left = [...received];
  return passed.every((entry) => {
    const at = left.findIndex((candidate) => sameData(candidate, entry));
    if (at < 0) return false;
    left.splice(at, 1);
    return true;
  });
}

/** Why one kind of a passed `provides` is refused, or `undefined`. */
function kindProblem(kind: ProvidesKind, received: unknown, passed: unknown): string | undefined {
  const before = listOf(received, kind) ?? [];
  const after = listOf(passed, kind);
  if (after === undefined) return `plugin.register: e.provides.${kind} must be a list`;
  if (!isSubset(before, after)) {
    return `plugin.register: e.provides.${kind} holds an entry the event did not`;
  }
  return FIXED[kind] && after.length < before.length
    ? `plugin.register: e.provides.${kind} cannot be narrowed`
    : undefined;
}

/**
 * Why the `provides` a hook passes to `next` is refused, comparing it with the one it received;
 * `undefined` when it fits, or when it is left out (then it is restored).
 */
export function providesProblem(received: unknown, passed: unknown): string | undefined {
  const given = isRecord(passed) ? passed.provides : undefined;
  if (given === undefined) return undefined;
  if (!isRecord(given)) return 'plugin.register: e.provides must be an object';
  const before = isRecord(received) ? received.provides : undefined;
  for (const kind of PROVIDES_KINDS) {
    const problem = kindProblem(kind, before, given);
    if (problem !== undefined) return problem;
  }
  return undefined;
}

/** The event a hook passed on, with a `provides` it left out restored from the one it received. */
export function withProvides(received: unknown, passed: unknown): unknown {
  if (!isRecord(passed) || passed.provides !== undefined || !isRecord(received)) return passed;
  return { ...passed, provides: received.provides };
}

/** The entries of `received` that `kept` no longer lists, by kind; only the kinds that lost one. */
export function leftOutOf(
  received: PluginProvides,
  kept: unknown,
): Partial<Record<ProvidesKind, readonly unknown[]>> {
  const out: Partial<Record<ProvidesKind, readonly unknown[]>> = {};
  for (const kind of PROVIDES_KINDS) {
    const remaining = [...(listOf(kept, kind) ?? received[kind])];
    const gone = received[kind].filter((entry) => {
      const at = remaining.findIndex((candidate) => sameData(candidate, entry));
      if (at < 0) return true;
      remaining.splice(at, 1);
      return false;
    });
    if (gone.length > 0) out[kind] = gone;
  }
  return out;
}
