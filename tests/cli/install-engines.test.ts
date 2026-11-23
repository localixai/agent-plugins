/**
 * Spec §2.4 (`engines`; changelog 2.1.323) and §17.4 (plugins/cli-reference.md, "plugin install"):
 * a plugin whose `engines["claude-code"]` range excludes the running client does not install; one
 * read from a local folder installs, the success line followed by `. Note: "<name>" needs Claude Code
 * <range>, this is <version>, but loads anyway because it's read from a local folder`.
 */
import { describe, expect, test } from 'bun:test';
import { effectsOf, lastJson, printed, run } from '../support/install.ts';
import { spec } from '../support/spec.ts';

const engines = (range: unknown, localFolder = false) => ({
  found: { engines: { engines: { 'claude-code': range }, clientVersion: '2.1.330', localFolder } },
});

spec('manifest.field.engines', () => {
  test('a range that fits installs as usual', async () => {
    expect(printed(await run(engines('>=2.1.0'))).stdout).toEqual([
      'Successfully installed plugin: formatter@m (scope: user)',
    ]);
  });

  test('a range that excludes the client refuses the install, and nothing is installed', async () => {
    const found = await run(engines('>=2.1.400'));
    expect(effectsOf(found)).toBeUndefined();
    expect(printed(found).exitCode).toBe(1);
  });

  test('read from a local folder, it installs with the note after the success line', async () => {
    const found = await run(engines('>=9.0.0', true));
    expect(printed(found).stdout).toEqual([
      `Successfully installed plugin: formatter@m (scope: user). Note: "formatter" needs Claude Code >=9.0.0, this is 2.1.330, but loads anyway because it's read from a local folder`,
    ]);
    expect(effectsOf(found)?.install).toBe(true);
  });

  test('a value that is not a range is ignored: it installs', async () => {
    expect(printed(await run(engines('banana'))).exitCode).toBe(0);
  });
});

describe('judgment J2.10: the refusal the spec does not word', () => {
  test('names the range and the version, with failureCode engine_range', async () => {
    const found = await run(engines('>=2.1.400'));
    expect(printed(found).stderr).toEqual([
      'Failed to install plugin "formatter@m": "formatter" needs Claude Code >=2.1.400, this is 2.1.330',
    ]);
    expect(lastJson(found)).toMatchObject({ outcome: 'failed', failureCode: 'engine_range' });
  });
});
