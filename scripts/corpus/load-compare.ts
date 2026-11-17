/**
 * `bun run corpus:load`: what the kit loads from every corpus plugin, compared with what Claude Code
 * loads as the spec recorded it (`load-oracle.ts`), for the release `spec/SOURCE.json` pins: whether
 * the plugin loads, and the names of its skills (commands included), agents, hook events, MCP
 * servers and LSP servers. A plugin without a recorded verdict is counted apart. Prints the
 * agreement per field and writes every difference to `.cache/corpus/load-report.json`.
 */
import { join } from 'node:path';
import { readSource } from '../spec/source.ts';
import { readCorpusTargets, REPORTS_DIR } from './location.ts';
import { kitInventory } from './load-kit.ts';
import { loadOracle } from './load-oracle.ts';
import { lookupName } from './plugin-name.ts';
import { pool } from './pool.ts';

const FIELDS = ['loaded', 'skills', 'agents', 'hookEvents', 'mcpServers', 'lspServers'] as const;
const same = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

const version = (await readSource()).release;
const plugins = await readCorpusTargets();
let done = 0;
const answered = await pool(plugins, 8, async (entry) => {
  const name = await lookupName(entry.dir);
  const [oracle, kit] = await Promise.all([
    loadOracle(version, entry.key),
    kitInventory(entry.dir, name).catch((error: unknown) => ({ crashed: String(error) })),
  ]);
  done += 1;
  if (done % 100 === 0) console.error(`${done}/${plugins.length}`);
  if (oracle === undefined) return undefined;
  if ('crashed' in kit) return { plugin: entry.key, crashed: kit.crashed, differs: ['crash'] };
  const differs = FIELDS.filter(
    (field) =>
      !(oracle.loaded && kit.loaded
        ? same(oracle[field], kit[field])
        : oracle.loaded === kit.loaded),
  );
  return { plugin: entry.key, differs, oracle, kit };
});

const rows = answered.filter((row) => row !== undefined);
const pct = (n: number): string => `${((100 * n) / rows.length).toFixed(1)}%`;
console.log(`Claude Code ${version} loading verdicts over ${rows.length} plugins`);
console.log(`  without a recorded verdict: ${answered.length - rows.length}`);
console.log(
  `  whole inventory agrees: ${pct(rows.filter((row) => row.differs.length === 0).length)}`,
);
for (const field of [...FIELDS, 'crash'] as const)
  console.log(
    `  ${field} differs: ${rows.filter((row) => row.differs.some((one) => one === field)).length}`,
  );
await Bun.write(
  join(REPORTS_DIR, 'load-report.json'),
  `${JSON.stringify(
    rows.filter((row) => row.differs.length > 0),
    null,
    2,
  )}\n`,
);
