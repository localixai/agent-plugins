/**
 * The guard keeps a connected managed MCP server's tool list. Agent Plugins Extended 14-policy-mods.md
 * §14.9.2 item 3: where the guard loads, a user's mod cannot rewrite "the tool list and tool
 * descriptions of a managed MCP server that has connected". The host names those tools
 * (`ManagedModData.managedTools`): the guard puts them back, as the host gave them, in the pool the
 * client's own `tool.list` shapes, and a user's `tool.offer` hook does not run for one of them.
 */
import { describe, expect, test } from 'bun:test';
import { apiHarness } from '../../support/mods-api.ts';
import { fn, record } from '../../support/mods-chain.ts';
import { managed, policy, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

const MANAGED = {
  name: 'mcp__corp__search',
  description: 'Search the company wiki',
  mcp: true,
};
const OURS = { name: 'Read', description: 'Read a file', mcp: false };
const POOL = [OURS, MANAGED];

type Tool = { readonly name: string };
const named = (value: unknown): readonly Tool[] => {
  const list = record(value).value;
  return Array.isArray(list) ? list : [];
};

/** A user's mod that drops the managed tool, rewrites the rest and adds one of its own. */
const reshape = fn('tool.list', async (_$, e, next) => {
  const list = named(await next(e)).filter((tool) => tool.name !== MANAGED.name);
  return {
    value: [
      ...list.map((tool) => ({ ...tool, description: 'changed' })),
      { name: 'mcp__mine__x', description: 'mine', mcp: true },
    ],
  };
});

async function hostWith(given: Parameters<typeof session>[1]) {
  const s = session({}, { firesSessionStart: false, ...given });
  await s.host.load([]);
  const { api } = await apiHarness();
  return { s, api };
}

describe('a managed MCP server that has connected', () => {
  spec('policy.mods.Know what happens by default', () => {
    test("a user's mod cannot drop or rewrite its tools in the client's tool pool", async () => {
      const { s, api } = await hostWith({ policy: managed({}), managedTools: () => [MANAGED] });
      s.host.chain.register({ name: 'user', api }, 'user', [reshape]);
      const pool = await s.host.fire(
        'tool.list',
        {},
        { core: () => Promise.resolve({ value: POOL }) },
      );
      expect(named(pool)).toEqual([
        { ...OURS, description: 'changed' },
        { name: 'mcp__mine__x', description: 'mine', mcp: true },
        MANAGED,
      ]);
    });

    test("a user's mod that rewrites a managed tool in place gets the host's copy back", async () => {
      const { s, api } = await hostWith({ policy: managed({}), managedTools: () => [MANAGED] });
      s.host.chain.register({ name: 'user', api }, 'user', [
        fn('tool.list', async (_$, e, next) => ({
          value: named(await next(e)).map((tool) => ({ ...tool, description: 'mine now' })),
        })),
      ]);
      const pool = await s.host.fire(
        'tool.list',
        {},
        { core: () => Promise.resolve({ value: POOL }) },
      );
      expect(named(pool)).toEqual([{ ...OURS, description: 'mine now' }, MANAGED]);
    });

    test("a user's tool.offer hook does not run for a managed tool, and runs for any other", async () => {
      const { s, api } = await hostWith({ policy: managed({}), managedTools: () => [MANAGED] });
      const seen: string[] = [];
      s.host.chain.register({ name: 'user', api }, 'user', [
        fn('tool.offer', (_$, e) => (seen.push(String(record(e).tool)), { isOffered: false })),
      ]);
      const provider = { plugin: 'engine', tier: 'core' };
      const offered = { core: () => Promise.resolve({ isOffered: true }) };
      const managedOffer = await s.host.fire(
        'tool.offer',
        { tool: MANAGED.name, provider },
        offered,
      );
      const otherOffer = await s.host.fire('tool.offer', { tool: OURS.name, provider }, offered);
      expect(managedOffer).toEqual({ isOffered: true });
      expect(otherOffer).toEqual({ isOffered: false });
      expect(seen).toEqual([OURS.name]);
    });
  });

  test('judgment: where the guard does not load, the host fact changes nothing', async () => {
    const { s, api } = await hostWith({ policy: policy(), managedTools: () => [MANAGED] });
    s.host.chain.register({ name: 'user', api }, 'user', [reshape]);
    const pool = await s.host.fire(
      'tool.list',
      {},
      { core: () => Promise.resolve({ value: POOL }) },
    );
    expect(named(pool).map((tool) => tool.name)).toEqual(['Read', 'mcp__mine__x']);
  });
});
