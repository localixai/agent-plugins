/** Agent Plugins Extended §10.43.2 (troubleshoot.md § `stopped · type a message to continue`, the 2.1.330 wording); api.md § Calls and background work after the user interrupts a turn. */
import { expect, test } from 'bun:test';
import type { ModNext, ModsApi } from '../../../src/mods/types.ts';
import { STOPPED_TOAST } from '../../../src/mods/api/index.ts';
import { fakePorts } from '../../../src/testing/mods/index.ts';
import { inline, modPlugin, registerOf, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

type Next = ModNext<unknown, Promise<unknown> | AsyncGenerator<unknown, unknown>>;
const calls = [
  [
    'prompt.submit',
    registerOf((on) => {
      on('classic.Stop', async ($: ModsApi, e: unknown, next: Next) => {
        await $.prompt.submit({ text: 'continue' });
        return next(e);
      });
    }),
  ],
  [
    'command.run',
    registerOf((on) => {
      on('classic.Stop', async ($: ModsApi, e: unknown, next: Next) => {
        await $.command.run({ command: 'review', args: '' });
        return next(e);
      });
    }),
  ],
  [
    'agent.spawn',
    registerOf((on) => {
      on('classic.Stop', async ($: ModsApi, e: unknown, next: Next) => {
        await $.agent.spawn({ prompt: 'review' });
        return next(e);
      });
    }),
  ],
  [
    'tool.call',
    registerOf((on) => {
      on('classic.Stop', async ($: ModsApi, e: unknown, next: Next) => {
        await $.tool.call({ tool: 'Bash', command: 'ls', run_in_background: true });
        return next(e);
      });
    }),
  ],
] as const;

spec('mods.troubleshoot.stopped · type a message to continue', () => {
  for (const [name, register] of calls) {
    test(`${name} refused after a stop raises the engine toast`, async () => {
      const base = fakePorts().ports;
      const s = session(
        { resumer: register },
        { ports: { ...base, session: { ...base.session, turns: () => Promise.resolve(1) } } },
      );
      const loaded = await s.host.load([modPlugin(inline('resumer'), register)]);
      expect(loaded.lines).toEqual([]);
      expect(loaded.loaded).toHaveLength(1);
      s.host.input.interrupt();
      await s.host.fire('classic.Stop', { hook_event_name: 'Stop' });
      expect(s.host.ui.toasts.visible().map((toast) => toast.text)).toEqual([
        'stopped · type a message to continue',
      ]);
      expect(STOPPED_TOAST).toBe('stopped · type a message to continue');
    });
  }
});

test('a refused session.send reports its own reason and raises no stopped-work toast', async () => {
  const register = registerOf((on) => {
    on('classic.Stop', async ($: ModsApi, e: unknown, next: Next) => {
      await $.session.send({ to: 'missing', text: 'ping' });
      return next(e);
    });
  });
  const base = fakePorts().ports;
  const s = session(
    { sender: register },
    {
      ports: {
        ...base,
        session: {
          ...base.session,
          send: () => Promise.resolve({ isSent: false, reason: 'target finished' }),
        },
      },
    },
  );
  await s.host.load([modPlugin(inline('sender'), register)]);
  s.host.input.interrupt();
  await s.host.fire('classic.Stop', { hook_event_name: 'Stop' });
  expect(s.host.ui.toasts.visible()).toEqual([]);
});
