/**
 * hooks.md (2.1.330) § "PostToolUse": "In the main conversation, PostToolUse hooks from managed
 * settings also run after some failed calls", and § "PostToolUse input": for a failed call
 * `tool_response` is the error text. Agent Plugins Extended §7.5.11, Judgment J7.215: which failed
 * calls is the host's; hooks from any other source never run on a failure.
 */
import { expect, test } from 'bun:test';
import { createHookEngine, createHookRegistry } from '../../src/hooks/index.ts';
import type { HookSource } from '../../src/hooks/index.ts';
import { entry, FACTS, session, settingsSource } from '../support/hooks-select.ts';
import { fakeEnvironment, locations, scriptedRunner } from '../support/ports.ts';
import { spec } from '../support/spec.ts';
import { stdinJson } from '../support/stdin.ts';

const managed: HookSource = { kind: 'settings', label: '[settings]', managed: true };

const fireFailed = async (failedCall: boolean) => {
  const runner = scriptedRunner({ exitCode: 0 });
  const engine = createHookEngine({
    commands: runner,
    locations: locations(),
    environment: fakeEnvironment(),
  });
  const registry = createHookRegistry([
    entry('PostToolUse', { source: managed, handler: { type: 'command', command: './audit.sh' } }),
    entry('PostToolUse', {
      source: settingsSource,
      handler: { type: 'command', command: './fmt.sh' },
    }),
  ]);
  const outcome = await engine.dispatch(registry, {
    event: 'PostToolUse',
    facts: { ...FACTS.PostToolUse, tool_response: 'Error: file not found' },
    session: session(),
    signal: new AbortController().signal,
    ...(failedCall ? { failedCall: true } : {}),
  });
  return { outcome, runner };
};

spec('hooks.event.PostToolUse', () => {
  test('on a failed call only the hooks of managed settings run, and read the error text', async () => {
    const { outcome, runner } = await fireFailed(true);
    expect(outcome.runs).toHaveLength(1);
    expect(runner.calls).toHaveLength(1);
    const call = runner.calls[0];
    expect(call?.spec.command).toBe('./audit.sh');
    expect(call === undefined ? undefined : stdinJson(call)['tool_response']).toBe(
      'Error: file not found',
    );
  });

  test('on a successful call every source runs', async () => {
    const { outcome } = await fireFailed(false);
    expect(outcome.runs).toHaveLength(2);
  });
});
