/**
 * The user stops Claude. Agent Plugins Extended events.md §10.19.2 (`session.stop`): the event is
 * observe-only, the client raises it before the abort and waits on no hook, and work a hook queues in
 * it is refused as api.md §10.33.1 says (`the session was stopped and nobody has sent input since`).
 * `ModsHost.stop` is that door: it holds the session's input, then fires the event.
 */
import { describe, expect, test } from 'bun:test';
import { inline, modPlugin, registerOf, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

type Submitting = { prompt: { submit(input: unknown): Promise<unknown> } };
type Next = (e: unknown) => unknown;

const STOPPED = 'the session was stopped and nobody has sent input since';

/** A mod whose `session.stop` hook submits a prompt, and when that call settled, with what. */
async function stopped(isTurnRunning: boolean) {
  const { promise: queued, resolve } = Promise.withResolvers<unknown>();
  const seen: unknown[] = [];
  const register = registerOf((on) => {
    on('session.stop', async ($: Submitting, e: unknown, next: Next) => {
      seen.push(e);
      resolve(await $.prompt.submit({ text: 'carry on' }));
      return next(e);
    });
  });
  const s = session({ watcher: register });
  await s.host.load([modPlugin(inline('watcher'), register)]);
  const returned = s.host.stop(isTurnRunning);
  return { returned, queued: await queued, seen };
}

describe('ModsHost.stop', () => {
  spec('mods.event.session.stop', () => {
    test('a prompt a session.stop hook submits is refused with the stopped reason', async () => {
      const { queued, seen } = await stopped(true);
      expect(queued).toEqual({ drop: STOPPED });
      expect(seen).toEqual([{ isTurnRunning: true }]);
    });

    test('at an idle prompt too: the stop refuses what its hooks queue', async () => {
      const { queued, seen } = await stopped(false);
      expect(queued).toEqual({ drop: STOPPED });
      expect(seen).toEqual([{ isTurnRunning: false }]);
    });

    test('the stop waits on no hook: it returns at once, while a hook never ends', async () => {
      const register = registerOf((on) => {
        on('session.stop', () => new Promise<never>(() => undefined));
      });
      const s = session({ stuck: register });
      await s.host.load([modPlugin(inline('stuck'), register)]);
      expect(s.host.stop(true)).toBeUndefined();
    });
  });
});
