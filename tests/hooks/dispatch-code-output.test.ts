/**
 * code-mode.md (2.1.330) § "Move a check from a `UserPromptSubmit` hook": "On v2.1.328 or later,
 * `UserPromptSubmit` hooks don't receive code output, even when an administrator sets them in managed
 * settings"; § "Code output withheld by a hook": before 2.1.328 a `UserPromptSubmit` hook received
 * background code output as a prompt and a block withheld it, and a hook written for those releases
 * "doesn't block when `source` in its input is `poll_event`". Agent Plugins Extended §7.6.11.3 items 4
 * and 5, §7.5.8. The host marks code output it would pass as a prompt with
 * `DispatchRequest.fromCode`.
 */
import { expect, test } from 'bun:test';
import { createHookEngine, createHookRegistry } from '../../src/hooks/index.ts';
import type { HookSource } from '../../src/hooks/index.ts';
import type { CommandRun } from '../../src/ports/index.ts';
import { entry, FACTS, pluginSource, session, settingsSource } from '../support/hooks-select.ts';
import { fakeEnvironment, locations, scriptedRunner } from '../support/ports.ts';
import { spec } from '../support/spec.ts';
import { stdinJson } from '../support/stdin.ts';

const managed: HookSource = { kind: 'settings', label: '[settings]', managed: true };

const BLOCK = JSON.stringify({ decision: 'block', reason: 'no secrets in prompts' });

/** A check written for releases before 2.1.328: it blocks every prompt, code output included. */
const blocksAll = (): string => BLOCK;

/** The same check made to let code output through: it does not block a `poll_event` prompt. */
const letsPollEventsThrough = (run: CommandRun): string =>
  stdinJson(run)['source'] === 'poll_event' ? '{}' : BLOCK;

async function prompt(
  options: {
    readonly fromCode?: boolean;
    readonly source?: 'user' | 'poll_event';
    readonly check?: (run: CommandRun) => string;
  } = {},
) {
  const check = options.check ?? blocksAll;
  const runner = scriptedRunner(
    ...Array.from({ length: 3 }, () => (run: CommandRun) => ({ stdout: check(run) })),
  );
  const engine = createHookEngine({
    commands: runner,
    locations: locations(),
    environment: fakeEnvironment(),
  });
  const registry = createHookRegistry(
    // Three commands: identical settings handlers run once (src/hooks/select/dedupe.ts).
    [managed, settingsSource, pluginSource('guard')].map((source, index) =>
      entry('UserPromptSubmit', {
        source,
        handler: { type: 'command', command: `./check-${index}.sh` },
      }),
    ),
  );
  const outcome = await engine.dispatch(registry, {
    event: 'UserPromptSubmit',
    facts: {
      ...FACTS.UserPromptSubmit,
      prompt: 'deploy token: abc123',
      ...(options.source === undefined ? {} : { source: options.source }),
    },
    session: session(),
    signal: new AbortController().signal,
    ...(options.fromCode === true ? { fromCode: true } : {}),
  });
  return { outcome, runner };
}

spec('hooks.code-mode.Move a check from a UserPromptSubmit hook', () => {
  test('code output runs no UserPromptSubmit hook, managed, settings or plugin: it reaches Claude as is', async () => {
    const { outcome, runner } = await prompt({ fromCode: true, source: 'poll_event' });
    expect(runner.calls).toHaveLength(0);
    expect(outcome.runs).toHaveLength(0);
    expect(outcome.proceed).toBe(true);
    expect(outcome.block).toBeUndefined();
  });

  test('a prompt that is not code output still runs every UserPromptSubmit hook', async () => {
    const { outcome, runner } = await prompt();
    expect(runner.calls).toHaveLength(3);
    expect(outcome.proceed).toBe(false);
  });
});

spec('hooks.code-mode.Code output withheld by a hook', () => {
  test('a hook that blocks every prompt, as written for releases before 2.1.328, no longer withholds code output', async () => {
    const { outcome } = await prompt({ fromCode: true, source: 'poll_event', check: blocksAll });
    expect(outcome.proceed).toBe(true);
    expect(outcome.block).toBeUndefined();
    expect(outcome.additionalContext).toEqual([]);
    expect(outcome.systemMessages).toEqual([]);
  });

  test('a hook changed to let poll_event through reads the source and lets such a prompt reach Claude', async () => {
    const passed = await prompt({ source: 'poll_event', check: letsPollEventsThrough });
    expect(passed.runner.calls.map((run) => stdinJson(run)['source'])).toEqual([
      'poll_event',
      'poll_event',
      'poll_event',
    ]);
    expect(passed.outcome.proceed).toBe(true);
    const typed = await prompt({ source: 'user', check: letsPollEventsThrough });
    expect(typed.outcome.proceed).toBe(false);
    expect(typed.outcome.block?.reason).toBe('no secrets in prompts');
  });
});
