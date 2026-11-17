/**
 * errors.md, "Hook needs quotes around a path with a space" (Agent Plugins Extended §17.18.8): an
 * `asyncRewake` hook in shell form whose interpreter is followed by an unquoted path placeholder
 * that resolves to a path with a space does not run, and the conversation view is told
 * `<Event> hook needs quotes around a path with a space (<script>)`.
 */
import { expect, test } from 'bun:test';
import { resolveHookCommand } from '../../../src/entry/loading.ts';
import { hook, scopeOf } from '../../support/hook-scope.ts';
import { locations } from '../../support/ports.ts';
import { spec } from '../../support/spec.ts';

const spaced = scopeOf({ locations: locations({ plugins: '/Users/me/My Plugins' }) });
const command = 'python3 ${CLAUDE_PLUGIN_ROOT}/hooks/check.py';

spec('cli.errors.Hook needs quotes around a path with a space', () => {
  test('the hook does not run, and the notice names the event and the script', async () => {
    const found = await resolveHookCommand(hook({ command, asyncRewake: true }), spaced, {
      event: 'Stop',
      path: 'hooks.Stop[0].hooks[0]',
    });
    expect(found.resolved).toBeUndefined();
    expect(found.notice).toBe('Stop hook needs quotes around a path with a space (check.py)');
    expect(found.issues).toEqual([
      {
        severity: 'warning',
        path: 'hooks.Stop[0].hooks[0].command',
        message: 'Stop hook needs quotes around a path with a space (check.py)',
      },
    ]);
  });

  test('a quoted path, a path without a space, or a hook that is not asyncRewake runs', async () => {
    const quoted = 'python3 "${CLAUDE_PLUGIN_ROOT}/hooks/check.py"';
    for (const [handler, scope] of [
      [hook({ command: quoted, asyncRewake: true }), spaced],
      [hook({ command, asyncRewake: true }), scopeOf()],
      [hook({ command }), spaced],
      [hook({ command, args: [], asyncRewake: true }), spaced],
    ] as const) {
      const found = await resolveHookCommand(handler, scope, { event: 'Stop' });
      expect(found.resolved).toBeDefined();
      expect(found.notice).toBeUndefined();
    }
  });
});

test('the interpreters and placeholder spellings the kit recognises (judgment)', async () => {
  for (const line of ['node $CLAUDE_PLUGIN_ROOT/a.js', 'python3.12 ${CLAUDE_PLUGIN_ROOT}/a.py']) {
    const found = await resolveHookCommand(hook({ command: line, asyncRewake: true }), spaced);
    expect(found.notice).toStartWith('Hook needs quotes around a path with a space (a.');
  }
  const own = await resolveHookCommand(
    hook({ command: '${CLAUDE_PLUGIN_ROOT}/run.sh', asyncRewake: true }),
    spaced,
  );
  expect(own.notice).toBeUndefined();
});
