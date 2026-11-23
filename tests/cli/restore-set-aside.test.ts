/**
 * Spec §17.10.1 (`claude plugin restore-set-aside <plugin>@<marketplace> [--discard]`; plugins/
 * cli-reference.md) and §17.1.1 "Plugin arguments": the options and secrets set aside when managed
 * settings replaced a marketplace are restored, or deleted with `--discard`; an id with nothing set
 * aside, a bare name always, prints `No options or secrets are set aside for <plugin>.` and changes
 * nothing, exit `0` (Judgment J17.34); the command takes no `--json` (`error: unknown option
 * '--json'`, exit `1`). The success lines are the kit's (spec/judgments.md).
 */
import { describe, expect, test } from 'bun:test';
import { renderCommand } from '../../src/cli/result.ts';
import {
  planRestoreSetAside,
  restoredValues,
  type RestoreSetAsidePlan,
} from '../../src/cli/restore-set-aside.ts';
import { spec } from '../support/spec.ts';

const facts = { setAside: ['formatter@acme'] };
const human = (plan: RestoreSetAsidePlan) => renderCommand(plan.output, { json: false });

spec('cli.plugin.plugin restore-set-aside', () => {
  test('without --discard the set-aside values are restored under the id', () => {
    const plan = planRestoreSetAside({ plugin: 'formatter@acme' }, facts);
    expect(plan.action).toEqual({ kind: 'restore', id: 'formatter@acme' });
    expect(human(plan).exitCode).toBe(0);
  });

  test('with --discard they are deleted', () => {
    const plan = planRestoreSetAside({ plugin: 'formatter@acme', discard: true }, facts);
    expect(plan.action).toEqual({ kind: 'discard', id: 'formatter@acme' });
    expect(human(plan).exitCode).toBe(0);
  });

  test('an id with nothing set aside prints the line, changes nothing and exits 0', () => {
    const plan = planRestoreSetAside({ plugin: 'linter@acme' }, facts);
    expect(plan.action).toBeUndefined();
    expect(human(plan)).toEqual({
      stdout: ['No options or secrets are set aside for linter@acme.'],
      stderr: [],
      exitCode: 0,
    });
  });

  test('the command has no --json', () => {
    const plan = planRestoreSetAside({ plugin: 'formatter@acme', json: true }, facts);
    expect(plan.action).toBeUndefined();
    expect(renderCommand(plan.output, { json: true })).toEqual({
      stdout: [],
      stderr: ["error: unknown option '--json'"],
      exitCode: 1,
    });
  });

  test('restoring may keep a value the id already holds and drop the set-aside one', () => {
    expect(restoredValues({ token: 'new' }, { token: 'old', region: 'eu' })).toEqual({
      token: 'new',
      region: 'eu',
    });
  });
});

spec('cli.plugin.claude plugin commands.Plugin arguments', () => {
  test('restore-set-aside takes the full name@marketplace id: a bare name has nothing set aside', () => {
    const plan = planRestoreSetAside({ plugin: 'formatter', discard: true }, facts);
    expect(plan.action).toBeUndefined();
    expect(human(plan).stdout).toEqual(['No options or secrets are set aside for formatter.']);
  });
});

describe('the kit’s success lines', () => {
  test('restore and discard name the id', () => {
    expect(human(planRestoreSetAside({ plugin: 'formatter@acme' }, facts)).stdout).toEqual([
      'Restored the options and secrets set aside for formatter@acme.',
    ]);
    expect(
      human(planRestoreSetAside({ plugin: 'formatter@acme', discard: true }, facts)).stdout,
    ).toEqual(['Discarded the options and secrets set aside for formatter@acme.']);
  });
});
