/**
 * Spec §15.5.3 and Judgment J15.15: an entry whose display field has the wrong type, or whose
 * `homepage` is not a URL, is listed but invalid: `claude plugin install` of it fails with `This
 * plugin's marketplace entry is invalid: <field>: <problem>[, <field>: <problem>…]`, exit 1, and
 * nothing is installed. The marketplace's other entries install.
 */
import { describe, expect, test } from 'bun:test';
import { effectsOf, lastJson, plugin, printed, run } from '../support/install.ts';
import { spec } from '../support/spec.ts';

const PROBLEMS = [
  'description: Invalid input: expected string, received number',
  'homepage: Invalid URL',
];

const [id, entry] = plugin('formatter');
const catalog = [[id, { ...entry, invalid: PROBLEMS }] as const, plugin('linter')];

spec('marketplace.rule.Display fields', () => {
  test('installing the invalid entry fails with its problems and installs nothing', async () => {
    const found = await run({ catalog });
    expect(printed(found)).toEqual({
      stdout: [],
      stderr: [
        "This plugin's marketplace entry is invalid: description: Invalid input: expected string, received number, homepage: Invalid URL",
      ],
      exitCode: 1,
    });
    expect(effectsOf(found)).toBeUndefined();
  });

  test('another entry of the same marketplace installs', async () => {
    const found = await run({ catalog, args: { plugin: 'linter@m' } });
    expect(printed(found).exitCode).toBe(0);
    expect(effectsOf(found)?.id).toBe('linter@m');
  });
});

describe('judgment J15.15: the --json result', () => {
  test('is a failure with failureCode invalid_entry', async () => {
    expect(lastJson(await run({ catalog }))).toMatchObject({
      command: 'install',
      outcome: 'failed',
      failureCode: 'invalid_entry',
    });
  });
});
