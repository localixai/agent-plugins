/**
 * What a Claude Code 2.1.322 SESSION registers from a plugin, against the kit's session (E5a;
 * docs/TESTING.md, "Oracle runs"; docs/SPEC.md, "Judgments and deviations"). Each folder of
 * spec/oracle/runtime is a plugin of its own; its `expected.json` is what the spec's session oracle
 * (`claude -p` and the `init` event) recorded, ids only, synced by `bun run spec:sync`. A field the
 * kit gives differently on purpose is in tests/fixtures/runtime-judgments.json with the row of
 * spec/judgments.md. Outside `spec(...)`: an oracle result is never compatibility.
 */
import { describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { kitRegistry } from '../../scripts/oracle/kit-registry.ts';
import { caseFolders, readExpected, RUNTIME_DIR } from '../../scripts/oracle/recorded.ts';
import { compareRegistries, readJudgments } from '../../scripts/oracle/registry-diff.ts';

const cases = caseFolders(RUNTIME_DIR);

describe('the kit registers what Claude Code registers', () => {
  test('there are fixtures, each with an expectation', async () => {
    expect(cases.length).toBeGreaterThan(20);
    for (const name of cases) {
      expect(await readExpected(join(RUNTIME_DIR, name))).toBeDefined();
    }
  });

  test.each(cases)('%s', async (name) => {
    const dir = join(RUNTIME_DIR, name);
    const expected = await readExpected(dir);
    if (expected === undefined) throw new Error(`${name} has no expected.json`);
    const verdicts = compareRegistries(
      name,
      await kitRegistry(dir),
      expected,
      await readJudgments(),
    );
    expect(verdicts.filter((one) => one.status === 'differs')).toEqual([]);
  });
});
