/**
 * Agent Plugins Extended §7.1.4 item 2 (hooks.md, "Hooks in skills and agents", 2.1.330): a skill's
 * hooks keep running "until the user exits or leaves the conversation with a command such as
 * `/clear` or `/resume`, which removes them". The session sees those moments as the host's
 * `SessionEnd` dispatch (every reason: exit, `/clear`, an interactive `/resume`), after which the
 * skill hooks are gone, and as a `SessionStart` with source `clear` or `resume`, before which they
 * are gone (a host that dispatches no `SessionEnd`). The skill's own `SessionEnd` hook still runs on
 * that last `SessionEnd`.
 */
import { expect, test } from 'bun:test';
import { hostOf, loaded, SESSION, signal } from '../support/host.ts';
import { spec } from '../support/spec.ts';

const json = (value: unknown): string => JSON.stringify(value);

const HOOKS = [
  '  PreToolUse:',
  '    - matcher: "Bash"',
  '      hooks:',
  '        - type: command',
  '          command: ./scripts/check.sh',
  '  SessionEnd:',
  '    - hooks:',
  '        - type: command',
  '          command: ./scripts/end.sh',
].join('\n');

async function opened() {
  const plugin = await loaded({
    '.claude-plugin/plugin.json': json({ name: 'guarded' }),
    'skills/secure/SKILL.md': `---\ndescription: secure operations\nhooks:\n${HOOKS}\n---\nRun secure.`,
  });
  const { host, runner } = await hostOf({
    plugins: [{ id: 'guarded@acme', plugin }],
    runs: [{}, {}, {}, {}],
  });
  const session = host.openSession(SESSION);
  expect((await session.expandSkill('guarded:secure', { signal: signal() })).ok).toBe(true);
  return { session, runner };
}

const bash = { tool_name: 'Bash', tool_input: { command: 'ls' }, tool_use_id: 't1' };
const commands = (runner: { readonly calls: readonly { readonly spec: unknown }[] }) =>
  runner.calls.map((call) => Reflect.get(Object(call.spec), 'command'));

spec('hooks.behavior.Hooks in skills and agents', () => {
  for (const reason of ['clear', 'resume', 'prompt_input_exit', 'other'] as const) {
    test(`SessionEnd (${reason}) runs the skill's SessionEnd hook, then removes its hooks`, async () => {
      const { session, runner } = await opened();
      await session.dispatch('SessionEnd', { reason }, { signal: signal() });
      expect(commands(runner)).toEqual(['./scripts/end.sh']);
      await session.dispatch('PreToolUse', bash, { signal: signal() });
      expect(commands(runner)).toEqual(['./scripts/end.sh']);
    });
  }

  for (const source of ['clear', 'resume'] as const) {
    test(`SessionStart (${source}) without a SessionEnd before it removes them too`, async () => {
      const { session, runner } = await opened();
      await session.dispatch('SessionStart', { source }, { signal: signal() });
      await session.dispatch('PreToolUse', bash, { signal: signal() });
      expect(runner.calls).toHaveLength(0);
    });
  }

  test('a compaction keeps them: SessionStart (compact) is not leaving the conversation', async () => {
    const { session, runner } = await opened();
    await session.dispatch('SessionStart', { source: 'compact' }, { signal: signal() });
    await session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(commands(runner)).toEqual(['./scripts/check.sh']);
  });

  test('invoking the skill again in the new conversation registers them again', async () => {
    const { session, runner } = await opened();
    await session.dispatch('SessionStart', { source: 'clear' }, { signal: signal() });
    expect((await session.expandSkill('guarded:secure', { signal: signal() })).ok).toBe(true);
    await session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(commands(runner)).toEqual(['./scripts/check.sh']);
  });
});
