/**
 * `/mcp` names the mod that held a server back. Agent Plugins Extended 14-policy-mods.md §14.9.2 item
 * 3 ("a user's mod CAN keep a managed MCP server from connecting (a `mcp.connect` hook on the
 * client's own connect, §10.19.7), and `/mcp` then lists the server as held by that mod") and
 * events.md §10.19.7 ("a hook that answers without `next` holds the server back, and `/mcp` lists it
 * as held by that hook's mod"). The host learns that mod from `FireOptions.answered`: the mod whose
 * hook answered without calling `next`, told once the chain has its answer.
 */
import { describe, expect, test } from 'bun:test';
import type { HookOrigin } from '../../../src/protocol/mods-events.ts';
import { inline, modPlugin, registerOf, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

type Next = (e: unknown) => unknown;

const CONNECT = { server: 'corp', provider: { plugin: 'engine', tier: 'core' } };
const HOLD = { deny: 'not on this network' };

/** Fires the client's own `mcp.connect` over mods named by their hooks; what the host was told. */
async function connect(mods: Readonly<Record<string, (e: unknown, next: Next) => unknown>>) {
  const registers = Object.fromEntries(
    Object.entries(mods).map(([name, hook]) => [
      name,
      registerOf((on) => {
        on('mcp.connect', (_$: unknown, e: unknown, next: Next) => hook(e, next));
      }),
    ]),
  );
  const s = session(registers, { firesSessionStart: false });
  await s.host.load(
    Object.entries(registers).map(([name, register]) => modPlugin(inline(name), register)),
  );
  const told: HookOrigin[] = [];
  let connected = 0;
  const answer = await s.host.fire('mcp.connect', CONNECT, {
    core: () => ((connected += 1), Promise.resolve({ value: { isConnected: true } })),
    answered: (by) => told.push(by),
  });
  return { answer, told, connected };
}

describe('the mod that held a managed server back', () => {
  spec('policy.mods.Know what happens by default', () => {
    test('a hook that answers without next: the host is told its mod, and nothing connects', async () => {
      const { answer, told, connected } = await connect({ holder: () => HOLD });
      expect(answer).toEqual(HOLD);
      expect(connected).toBe(0);
      expect(told).toEqual([{ plugin: 'holder', tier: 'user' }]);
    });

    test('beneath a hook that passes on: the mod that answered is named, not the one above it', async () => {
      const { told } = await connect({ watcher: (e, next) => next(e), holder: () => HOLD });
      expect(told.map((by) => by.plugin)).toEqual(['holder']);
    });

    test('every hook passes on: the client connects, and the host is told no mod', async () => {
      const { told, connected } = await connect({ watcher: (e, next) => next(e) });
      expect(connected).toBe(1);
      expect(told).toEqual([]);
    });
  });
});
