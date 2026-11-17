/**
 * env-vars.md § Variables, `CLAUDE_CODE_SHELL_PREFIX` (Agent Plugins Extended §7.3.2.3 item 5,
 * Judgment J7.66): a shell-form Bash hook runs as `<prefix> '<command>'`, the command line one
 * shell-quoted argument; exec-form and PowerShell hooks run without it. The page is no requirement
 * of `spec/requirements.json`, so this is tested outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { runHandler } from '../../../../src/hooks/handlers/index.ts';
import { runContext, settingsCommand } from '../../../support/hooks-run.ts';
import { fakeEnvironment, scriptedRunner } from '../../../support/ports.ts';

const PREFIX = { CLAUDE_CODE_SHELL_PREFIX: '/path/to/logger.sh' };

async function specOf(
  hook: ReturnType<typeof settingsCommand>,
  variables: Readonly<Record<string, string>> = PREFIX,
) {
  const runner = scriptedRunner({});
  await runHandler(hook, runContext({ runner, environment: fakeEnvironment(variables) }));
  return runner.calls[0]?.spec;
}

describe('CLAUDE_CODE_SHELL_PREFIX (env-vars.md)', () => {
  test('a shell-form Bash hook runs through the prefix, its command one quoted argument', async () => {
    expect(
      await specOf(settingsCommand({ command: `echo "it's" \${CLAUDE_PROJECT_DIR}` })),
    ).toEqual({
      form: 'shell',
      command: `/path/to/logger.sh 'echo "it'\\''s" \${CLAUDE_PROJECT_DIR}'`,
      shell: 'bash',
    });
  });

  test('exec-form and PowerShell hooks start without it', async () => {
    expect(await specOf(settingsCommand({ command: 'tool', args: ['a'] }))).toEqual({
      form: 'exec',
      command: 'tool',
      args: ['a'],
    });
    expect(await specOf(settingsCommand({ command: 'Write-Host x', shell: 'powershell' }))).toEqual(
      { form: 'shell', command: 'Write-Host x', shell: 'powershell' },
    );
  });

  test('unset, a hook runs as written', async () => {
    expect(await specOf(settingsCommand({ command: 'echo hi' }), {})).toEqual({
      form: 'shell',
      command: 'echo hi',
      shell: 'bash',
    });
  });
});
