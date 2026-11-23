/**
 * The time a `ui.render` or `plugin.register` hook waits on `$` calls for one event, all together
 * (Agent Plugins Extended §10.11, reference.md "Limits", the row with no requirement id of its own):
 * "30 seconds for `ui.render`; 10 seconds for `plugin.register`, not counting `$.ui.ask` ... A hook
 * past it is skipped." Outside `spec(...)`: the row carries no requirement id.
 */
import { describe, expect, test } from 'bun:test';
import { deferred, flush, fn, harness } from '../../support/mods-chain.ts';

const waitingOn = (call: 'fs.read' | 'ui.ask') =>
  fn('plugin.register', async ($, e, next) => {
    const [namespace, method] = call.split('.');
    const member: unknown = Reflect.get(Reflect.get($, namespace ?? '') ?? {}, method ?? '');
    if (typeof member === 'function') await member('x');
    return next(e);
  });

describe('the time a hook waits on $ calls (§10.11)', () => {
  test('a plugin.register hook that waits 10 seconds on $ calls is skipped', async () => {
    const h = harness();
    let read = deferred();
    h.mod('policy', 'prepend', [waitingOn('fs.read')], {
      'fs.read': () => (read = deferred()).promise,
    });
    let done = false;
    const pending = h.fire('plugin.register', { name: 'other' }).then(() => (done = true));
    await flush();
    h.clock.advance(9999);
    await flush();
    expect(done).toBe(false);
    h.clock.advance(1);
    await pending;
    read.resolve();
    expect(h.reports.map((report) => report.line)).toEqual([
      'policy: plugin.register hook skipped: timed out: waited on $ calls past its 10000 ms limit',
    ]);
  });

  test('time inside $.ui.ask does not count for plugin.register', async () => {
    const h = harness();
    const asked = deferred();
    h.mod('policy', 'prepend', [waitingOn('ui.ask')], { 'ui.ask': () => asked.promise });
    const pending = h.fire('plugin.register', { name: 'other' });
    await flush();
    h.clock.advance(3_600_000);
    asked.resolve();
    expect(await pending).toEqual({ core: { name: 'other' } });
    expect(h.reports).toEqual([]);
  });

  test('a ui.render hook may wait 30 seconds on $ calls, and is skipped past them', async () => {
    const h = harness();
    let read = deferred();
    h.mod(
      'drawer',
      'user',
      [
        fn('ui.render', async ($, e, next) => {
          await $.fs.read('a');
          await $.fs.read('b');
          return next(e);
        }),
      ],
      { 'fs.read': () => (read = deferred()).promise },
    );
    let done = false;
    const pending = h.fire('ui.render', { component: 'Pane' }).then(() => (done = true));
    await flush();
    h.clock.advance(20_000);
    read.resolve();
    await flush();
    h.clock.advance(9999);
    await flush();
    expect(done).toBe(false);
    h.clock.advance(1);
    await pending;
    read.resolve();
    expect(h.reports.map((report) => report.reason)).toEqual([
      'timed out: waited on $ calls past its 30000 ms limit',
    ]);
  });

  test('other events have no such limit: a tool.call hook may wait on $ calls an hour', async () => {
    const h = harness();
    const read = deferred();
    h.mod(
      'my-mod',
      'user',
      [
        fn('tool.call', async ($, e, next) => {
          await $.fs.read('a');
          return next(e);
        }),
      ],
      { 'fs.read': () => read.promise },
    );
    const pending = h.fire('tool.call', { n: 1 });
    await flush();
    h.clock.advance(3_600_000);
    read.resolve();
    expect(await pending).toEqual({ core: { n: 1 } });
    expect(h.reports).toEqual([]);
  });
});
