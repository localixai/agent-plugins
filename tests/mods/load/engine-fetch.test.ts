/**
 * Claude Code's own `http.fetch` requests (Agent Plugins Extended §10.19.7, "Fields on Claude
 * Code's own `http.fetch` requests"; 2.1.321 on): an `http.fetch` event whose `next.origin` is
 * `{ plugin: 'engine', tier: 'core' }` is one of the client's own requests, and where the built-in
 * guard loads only a mod that counts as the organisation's gets these events. `ModsHost.fire`
 * fires as the client (`ENGINE_ORIGIN`). Its fields
 * (`e.init.compress`, `value.isStreaming`) are the host's to fill: the kit carries them as given.
 */
import { expect, test } from 'bun:test';
import { apiHarness } from '../../support/mods-api.ts';
import { fn } from '../../support/mods-chain.ts';
import { managed, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

const request = { url: 'https://api.anthropic.com/v1/messages', init: { compress: 'gzip' } };

spec("mods.reference.Fields on Claude Code's own http.fetch requests", () => {
  test("where the guard loads, the client's own requests reach the organization's mods, not a user's", async () => {
    const s = session({}, { policy: managed({}), firesSessionStart: false });
    await s.host.load([]);
    const trace: string[] = [];
    const { api } = await apiHarness();
    s.host.chain.register({ name: 'user-mod', api }, 'user', [
      fn('http.fetch', (_$, e, next) => (trace.push('user'), next(e))),
    ]);
    s.host.chain.register({ name: 'company-mod', api }, 'prepend', [
      fn(
        'http.fetch',
        (_$, e, next) => (trace.push(`organization ${JSON.stringify(e.init)}`), next(e)),
      ),
    ]);
    const answer = await s.host.fire('http.fetch', request, {
      core: () => (
        trace.push('core'),
        Promise.resolve({ value: { status: 200, isStreaming: true } })
      ),
    });
    expect(trace).toEqual(['organization {"compress":"gzip"}', 'core']);
    expect(answer).toEqual({ value: { status: 200, isStreaming: true } });
  });

  test("another mod's $.http.fetch still reaches a user's hook where the guard loads", async () => {
    const s = session({}, { policy: managed({}), firesSessionStart: false });
    await s.host.load([]);
    const trace: string[] = [];
    const { api } = await apiHarness();
    s.host.chain.register({ name: 'user-mod', api }, 'user', [
      fn('http.fetch', (_$, e, next) => (trace.push('user'), next(e))),
    ]);
    // The call of a mod that runs after the user's mod, as its `$.http.fetch` raises it.
    await s.host.chain.dispatch('http.fetch', request, {
      origin: { plugin: 'other-mod', tier: 'append' },
      signal: new AbortController().signal,
      report: () => undefined,
      core: () => (trace.push('core'), Promise.resolve({ value: { status: 200 } })),
    });
    expect(trace).toEqual(['user', 'core']);
  });

  test("without the guard a user mod hears the client's own requests too", async () => {
    const s = session({}, { firesSessionStart: false });
    await s.host.load([]);
    const trace: string[] = [];
    const { api } = await apiHarness();
    s.host.chain.register({ name: 'user-mod', api }, 'user', [
      fn('http.fetch', (_$, e, next) => (trace.push('user'), next(e))),
    ]);
    await s.host.fire('http.fetch', request, {
      core: () => Promise.resolve({ value: { status: 200 } }),
    });
    expect(trace).toEqual(['user']);
  });
});
