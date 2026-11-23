/**
 * Agent Plugins Extended §7.5.18 `SubagentStart`, §7.5.19 `SubagentStop`, §7.6.3 item 8 and §5.12.1:
 * for the built-in `web-fetch` subagent only the `SubagentStart` and `SubagentStop` hooks of managed
 * settings run; those of any other settings file and of plugins do not, while `PreToolUse` and
 * `PostToolUse` hooks matching `WebFetch` still run for its fetches (`DispatchRequest.webFetchAgent`).
 */
import { describe, expect, test } from 'bun:test';
import { createHookEngine, createHookRegistry } from '../../src/hooks/index.ts';
import type { DispatchRequest } from '../../src/hooks/index.ts';
import type { HookEvent } from '../../src/protocol/official.ts';
import {
  command,
  entry,
  FACTS,
  managedSettings,
  pluginSource,
  session,
  settingsSource,
} from '../support/hooks-select.ts';
import { fakeEnvironment, locations, scriptedRunner } from '../support/ports.ts';
import { spec } from '../support/spec.ts';

const engineWith = () => {
  const runner = scriptedRunner({ exitCode: 0 });
  const engine = createHookEngine({
    commands: runner,
    locations: locations(),
    environment: fakeEnvironment(),
  });
  return { runner, engine };
};

/** One hook per source on `event`: managed settings, user settings, a plugin. */
const registryFor = (event: HookEvent) =>
  createHookRegistry([
    entry(event, { source: managedSettings, handler: command('./managed.sh') }),
    entry(event, { source: settingsSource, handler: command('./user.sh') }),
    entry(event, { source: pluginSource(), handler: command('./plugin.sh') }),
  ]);

const ran = async <E extends HookEvent>(
  request: Omit<DispatchRequest<E>, 'session' | 'signal'>,
): Promise<readonly string[]> => {
  const { runner, engine } = engineWith();
  await engine.dispatch(registryFor(request.event), {
    ...request,
    session: session(),
    signal: new AbortController().signal,
  });
  return runner.calls.map((call) => call.spec.command);
};

const WEB_FETCH = { agent_id: 'a1', agent_type: 'web-fetch' };

spec(
  ['hooks.event.SubagentStart', 'agents.behavior.Project-level hooks for subagent events'],
  () => {
    describe('SubagentStart for the built-in web-fetch agent', () => {
      test('runs only the hooks of managed settings', async () => {
        const commands = await ran({
          event: 'SubagentStart',
          facts: WEB_FETCH,
          webFetchAgent: true,
        });
        expect(commands).toEqual(['./managed.sh']);
      });

      test('every source runs for another subagent', async () => {
        const commands = await ran({ event: 'SubagentStart', facts: FACTS.SubagentStart });
        expect(commands.toSorted()).toEqual(['./managed.sh', './plugin.sh', './user.sh']);
      });
    });
  },
);

spec('hooks.event.SubagentStop', () => {
  test('SubagentStop for the built-in web-fetch agent runs only the hooks of managed settings', async () => {
    const commands = await ran({
      event: 'SubagentStop',
      facts: { ...FACTS.SubagentStop, agent_type: 'web-fetch' },
      webFetchAgent: true,
    });
    expect(commands).toEqual(['./managed.sh']);
  });
});

spec('agents.behavior.Project-level hooks for subagent events', () => {
  test('the PreToolUse hooks matching WebFetch still run, from every source, for its fetches', async () => {
    const commands = await ran({
      event: 'PreToolUse',
      facts: { ...FACTS.PreToolUse, tool_name: 'WebFetch', tool_input: { url: 'https://x.dev' } },
      webFetchAgent: true,
    });
    expect(commands.toSorted()).toEqual(['./managed.sh', './plugin.sh', './user.sh']);
  });
});
