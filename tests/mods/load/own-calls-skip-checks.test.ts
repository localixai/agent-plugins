/**
 * Agent Plugins Extended §14.9.2 item 5 (plugins/mods/admin.md § Know what happens by default, "A
 * mod's own calls can skip your checks"): permission rules and `PreToolUse` hooks govern Claude's tool
 * calls, not a mod's `$.fs` and `$.process` calls; with `Read(.env)` denied, `$.fs.read('.env')`
 * still reads, and the guard, which judges Claude's calls at `tool.check`, does not stop them.
 */
import { expect, test } from 'bun:test';
import type { ModNext, ModsApi } from '../../../src/mods/types.ts';
import { fakePorts } from '../../../src/testing/mods/index.ts';
import { installed, managed, modPlugin, registerOf, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

type Next = ModNext<unknown, Promise<unknown>>;

spec("policy.mods.Know what happens by default.A mod's own calls can skip your checks", () => {
  test("with every call denied by the rules and the guard loaded, a user's mod still reads .env with $.fs.read", async () => {
    const seen: unknown[] = [];
    const register = registerOf((on) => {
      on('session.start', async ($: ModsApi, e: unknown, next: Next) => {
        seen.push(await $.fs.read('.env'));
        return next(e);
      });
    });
    const base = fakePorts().ports;
    const fs = {
      ...base.fs,
      read: () => Promise.resolve({ text: 'TOKEN=1', bytes: 7 }),
    };
    let asked = 0;
    const s = session(
      { reader: register },
      {
        policy: managed({}),
        ports: { ...base, fs },
        deny: {
          verdict: () => {
            asked += 1;
            return Promise.resolve('deny');
          },
        },
      },
    );
    const report = await s.host.load([modPlugin(installed('reader'), register)]);
    expect(report.loaded.map((mod) => mod.name)).toEqual(['cc-plugin-sec-default', 'reader']);
    expect(seen).toEqual(['TOKEN=1']);
    expect(asked).toBe(0);
  });
});
