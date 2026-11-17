/**
 * errors.md, "Hook needs quotes around a path with a space" (Agent Plugins Extended §17.18.8): an
 * `asyncRewake` shell-form hook whose interpreter is followed by an unquoted path placeholder that
 * resolves to a path with a space does not run, and the conversation view is told
 * `<Event> hook needs quotes around a path with a space (<script>)`: the hooks engine names the
 * event the handler is registered on and hands the line on as the handler's notice.
 */
import { expect, test } from 'bun:test';
import { runHandler } from '../../../../src/hooks/handlers/index.ts';
import { pluginCommand, runContext } from '../../../support/hooks-run.ts';
import { verdictOf } from '../../../support/hooks-verdict.ts';
import { locations, scriptedRunner } from '../../../support/ports.ts';
import { spec } from '../../../support/spec.ts';

const NOTICE = 'Stop hook needs quotes around a path with a space (check.py)';

spec('cli.errors.Hook needs quotes around a path with a space', () => {
  test('the hook does not run, and the notice names the event it is registered on', async () => {
    const runner = scriptedRunner({});
    const hook = pluginCommand(
      { command: 'python3 ${CLAUDE_PLUGIN_ROOT}/hooks/check.py', asyncRewake: true },
      'Stop',
    );
    const result = await runHandler(
      hook,
      runContext({ runner, locations: locations({ plugins: '/Users/me/My Plugins' }) }),
    );
    expect(runner.calls).toEqual([]);
    expect(result.run).toEqual({ kind: 'skipped', reason: NOTICE, notice: NOTICE });
    expect(result.issues.map(({ message }) => message)).toContain(NOTICE);
  });

  test('the skipped run carries the line to the conversation view as its notice', () => {
    const verdict = verdictOf('Stop', { kind: 'skipped', reason: NOTICE, notice: NOTICE });
    expect(verdict.status).toBe('skipped');
    expect(verdict.notice?.message).toBe(NOTICE);
  });
});
