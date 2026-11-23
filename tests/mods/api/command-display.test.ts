/**
 * Hiding command text from Claude. Agent Plugins Extended api.md §10.30.1, "Hiding command text from
 * Claude" (2.1.326): a `command.run` hook that answers without calling `next` and returns `{ text,
 * isTextDisplayOnly: true }`, for a command registered without `immediate`, has its text printed and
 * not given to the model; otherwise the model can read it. The kit keeps the flag only where it
 * stands (`displayOnlyReply`), so a host keeps from the model exactly the answers that carry it.
 */
import { expect, test } from 'bun:test';
import { displayOnlyReply } from '../../../src/mods/api/index.ts';
import { fn } from '../../support/mods-chain.ts';
import { apiHarness } from '../../support/mods-api.ts';
import { inline, modPlugin, registerOf, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

type Registered = { command: { register(command: unknown): Promise<unknown> } };
type Next = (e: unknown) => unknown;

const HIDDEN = { text: 'secret tally', isTextDisplayOnly: true };

spec('mods.apiGuide.Hide command text from Claude', () => {
  test('the flag stands only for a hook that answered without next, on a command registered without immediate', () => {
    expect(displayOnlyReply(HIDDEN, { answeredByHook: true, immediate: false })).toEqual(HIDDEN);
    for (const facts of [
      { answeredByHook: false, immediate: false },
      { answeredByHook: true, immediate: true },
    ]) {
      expect(displayOnlyReply(HIDDEN, facts)).toEqual({ text: 'secret tally' });
    }
    expect(displayOnlyReply({ text: 'x' }, { answeredByHook: true, immediate: false })).toEqual({
      text: 'x',
    });
  });

  test('$.command.run of a mod command: answered without next it keeps the flag; registered immediate, or passed on, it does not', async () => {
    const h = await apiHarness({
      ports: {
        commands: {
          builtIns: () => [],
          run: () => Promise.resolve(HIDDEN),
        },
      },
    });
    const owner = await h.mod('owner', 'prepend', [
      fn('command.run', (_$, e, next) => (e.command === 'help' ? next(e) : HIDDEN)),
    ]);
    await owner.api.command.register({ name: 'tally' });
    await owner.api.command.register({ name: 'now', immediate: true });
    expect(await h.api.command.run({ command: 'tally' })).toEqual(HIDDEN);
    expect(await h.api.command.run({ command: 'now' })).toEqual({ text: 'secret tally' });
    expect(await h.api.command.run({ command: 'help' })).toEqual({ text: 'secret tally' });
  });

  test('a command the user typed (ModsHost.runCommand): the same rule as $.command.run', async () => {
    const register = registerOf((on) => {
      on('session.start', async ($: Registered, e: unknown, next: Next) => {
        await $.command.register({ name: 'tally' });
        await $.command.register({ name: 'now', immediate: true });
        return next(e);
      });
      on('command.run', (_$: unknown, e: unknown, next: Next) =>
        typeof e === 'object' && e !== null && 'command' in e && e.command === 'help'
          ? next(e)
          : HIDDEN,
      );
    });
    const s = session({ owner: register });
    await s.host.load([modPlugin(inline('owner'), register)]);
    const core = { core: () => Promise.resolve(HIDDEN) };
    expect(await s.host.runCommand('tally', '', core)).toEqual(HIDDEN);
    expect(await s.host.runCommand('now', '', core)).toEqual({ text: 'secret tally' });
    expect(await s.host.runCommand('help', '', core)).toEqual({ text: 'secret tally' });
  });
});

test('judgment: a reply that is not an object is left as it is', () => {
  expect(displayOnlyReply(undefined, { answeredByHook: true, immediate: false })).toBeUndefined();
});
