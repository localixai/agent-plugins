/**
 * hooks.md § Limitations (an async `SessionStart` hook "runs every time the event fires"; Claude Code
 * skips its output when it repeats what Claude last received from that hook, unless a synchronous
 * `SessionStart` hook added other text since, and delivers it again the first time after a
 * compaction or `/clear`). Through `session.takeBackground()`.
 */
import { expect, test } from 'bun:test';
import { createPluginHost } from '../../src/host/index.ts';
import { settleDeliveries } from '../../src/hooks/index.ts';
import { loaded, SESSION, signal } from '../support/host.ts';
import { pendingRunner } from '../support/hooks-run.ts';
import { fakeEnvironment, locations, recordingHttp } from '../support/ports.ts';
import { spec } from '../support/spec.ts';

const json = (value: unknown): string => JSON.stringify(value);
const say = (text: string) =>
  json({ hookSpecificOutput: { hookEventName: 'SessionStart', additionalContext: text } });

async function sessionWith(handlers: readonly object[]) {
  const plugin = await loaded({
    '.claude-plugin/plugin.json': json({ name: 'ctx' }),
    'hooks/hooks.json': json({ hooks: { SessionStart: [{ hooks: handlers }] } }),
  });
  const runner = pendingRunner();
  const host = createPluginHost({
    plugins: [{ id: 'ctx@acme', plugin }],
    ports: {
      commands: runner,
      http: recordingHttp(),
      locations: locations({ directories: ['/plugins/ctx@acme', '/work/app'] }),
      environment: fakeEnvironment(),
    },
  });
  const session = host.openSession(SESSION);
  /** Fires SessionStart and finishes its runs with `outputs`, then returns what the queue hands on. */
  const start = async (
    source: 'startup' | 'resume' | 'clear' | 'compact',
    outputs: readonly string[],
  ) => {
    const first = runner.calls.length;
    const outcome = session.dispatch('SessionStart', { source }, { signal: signal() });
    for (let tick = 0; tick < 50 && runner.calls.length < first + outputs.length; tick += 1) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
    outputs.forEach((stdout, index) => runner.finish(first + index, { stdout }));
    await settleDeliveries((await outcome).background);
    await new Promise((resolve) => setTimeout(resolve, 0));
    return session.takeBackground().additionalContext;
  };
  return start;
}

const ASYNC = { type: 'command', command: './ctx.sh', async: true };

spec('hooks.behavior.Limitations', () => {
  test('a repeat of what the hook last gave is skipped; a change is delivered', async () => {
    const start = await sessionWith([ASYNC]);
    expect(await start('startup', [say('branch main')])).toEqual(['branch main']);
    expect(await start('resume', [say('branch main')])).toEqual([]);
    expect(await start('resume', [say('branch dev')])).toEqual(['branch dev']);
  });

  test('after a compaction or /clear the first run delivers again', async () => {
    const start = await sessionWith([ASYNC]);
    expect(await start('startup', [say('branch main')])).toEqual(['branch main']);
    expect(await start('compact', [say('branch main')])).toEqual(['branch main']);
    expect(await start('clear', [say('branch main')])).toEqual(['branch main']);
  });

  test('a synchronous SessionStart hook that added text since lets the repeat through', async () => {
    const start = await sessionWith([ASYNC, { type: 'command', command: './sync.sh' }]);
    expect(await start('startup', [say('branch main'), say('sync')])).toEqual(['branch main']);
    expect(await start('resume', [say('branch main'), say('sync')])).toEqual(['branch main']);
  });
});
