/**
 * `bun run oracle:runtime`: for each fixture of spec/oracle/runtime, the kit's session
 * (`kit-registry.ts`) against what Claude Code's session registered, as the spec recorded it in the
 * fixture's `expected.json` (`recorded.ts`), per case and field. Exits 1 on a difference that is
 * not recorded in tests/fixtures/runtime-judgments.json. The same comparison runs in `bun test`
 * (tests/host/runtime-oracle.test.ts); this prints every field.
 */
import { join } from 'node:path';
import { kitRegistry } from './kit-registry.ts';
import { caseFolders, readExpected, RUNTIME_DIR } from './recorded.ts';
import { compareRegistries, readJudgments, type FieldVerdict } from './registry-diff.ts';

const shown = (value: unknown): string => JSON.stringify(value);
const label = (status: FieldVerdict['status']): string =>
  status === 'differs' ? 'DIFFERS' : status === 'judged' ? 'judged' : 'ok';

const judgments = await readJudgments();
const outcomes = await Promise.all(
  caseFolders(RUNTIME_DIR).map(async (name) => {
    const dir = join(RUNTIME_DIR, name);
    const expected = await readExpected(dir);
    const verdicts =
      expected === undefined
        ? []
        : compareRegistries(name, await kitRegistry(dir), expected, judgments);
    return { name, verdicts };
  }),
);

let failures = 0;
for (const { name, verdicts } of outcomes) {
  if (verdicts.length === 0) {
    failures++;
    console.log(`${name}: no expected.json (the spec records it: \`bun run oracle:record\`)`);
    continue;
  }
  failures += verdicts.filter((verdict) => verdict.status === 'differs').length;
  console.log(`${name}: ${verdicts.map((one) => `${one.field}=${label(one.status)}`).join(' ')}`);
  for (const verdict of verdicts.filter((one) => one.status !== 'agrees')) {
    console.log(
      `    ${verdict.field}: oracle ${shown(verdict.expected)} kit ${shown(verdict.kit)}` +
        (verdict.row === undefined ? '' : `  (judgment: ${verdict.row})`),
    );
  }
}
const cells = outcomes.flatMap((outcome) => outcome.verdicts);
const count = (status: FieldVerdict['status']): number =>
  cells.filter((one) => one.status === status).length;
console.log(
  `\n${outcomes.length} cases, ${cells.length} fields: ${count('agrees')} agree, ${count('judged')} judged, ${count('differs')} differ`,
);
process.exit(failures > 0 ? 1 : 0);
