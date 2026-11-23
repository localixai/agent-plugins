/**
 * Spec §17.2.2, the two booleans `install` adds for a plugin whose marketplace entry installs it by
 * running a command (§15.6.6, §17.4.2; 2.1.330): `commandPrinted`, `true` when the install
 * succeeded and the command wrote to stderr, the human line then a warning showing what it wrote;
 * `commandFailed`, `true` when the command exited with an error, was killed or timed out. The
 * warning's and the failure's wording and the `command_failed` code are the kit's
 * (spec/judgments.md).
 */
import { describe, expect, test } from 'bun:test';
import { concludeCommandInstall } from '../../src/cli/install-command-outcome.ts';
import { commandSucceeded, renderCommand } from '../../src/cli/result.ts';
import { spec } from '../support/spec.ts';

const planned = {
  output: commandSucceeded(
    'install',
    'Successfully installed plugin: tool@acme (scope: user)\n1 userConfig option not yet set',
    { fields: { pluginId: 'tool@acme', scope: 'user' } },
  ),
  effects: { id: 'tool@acme', scope: 'user' as const },
};

const json = (output: Parameters<typeof renderCommand>[0]) =>
  JSON.parse(renderCommand(output, { json: true }).stdout.at(-1) ?? '{}');

spec('cli.plugin.JSON result format.commandPrinted', () => {
  test('true when the install succeeded and the command wrote to stderr', () => {
    const output = concludeCommandInstall(planned, { stderr: 'fetching…\n' });
    expect(json(output)).toMatchObject({
      outcome: 'ok',
      pluginId: 'tool@acme',
      commandPrinted: true,
      commandFailed: false,
    });
  });

  test('the human line is then a warning that shows what the command wrote', () => {
    const output = concludeCommandInstall(planned, { stderr: 'fetching…\n' });
    expect(renderCommand(output, { json: false }).stdout).toEqual([
      'Installed tool@acme (scope: user) with a warning: its install command wrote to stderr:\nfetching…\n1 userConfig option not yet set',
    ]);
  });

  test('false, and the success line unchanged, when the command wrote nothing to stderr', () => {
    const output = concludeCommandInstall(planned, { stderr: '' });
    expect(json(output)).toMatchObject({ commandPrinted: false, commandFailed: false });
    expect(json(output).message).toStartWith('Successfully installed plugin: tool@acme');
  });
});

spec('cli.plugin.JSON result format.commandFailed', () => {
  test('true when the command exited with an error, was killed or timed out: the install fails', () => {
    const output = concludeCommandInstall(planned, {
      stderr: 'boom',
      failure: 'The command exited with code 2; it must exit 0',
    });
    const rendered = renderCommand(output, { json: false });
    expect(rendered.exitCode).toBe(1);
    expect(json(output)).toMatchObject({
      outcome: 'failed',
      commandPrinted: false,
      commandFailed: true,
      failureCode: 'command_failed',
    });
    expect(rendered.stderr).toEqual([
      'Failed to install plugin "tool@acme": The command exited with code 2; it must exit 0',
    ]);
  });
});

describe('a planned output that is no success', () => {
  test('is returned as it is', () => {
    const refused = { ...planned, output: { kind: 'usage-error' as const, reason: 'x' } };
    expect(concludeCommandInstall(refused, { stderr: 'a' })).toEqual(refused.output);
  });
});
