/**
 * `bun run corpus:runtime [--release x.y.z] [--limit N] [--only a,b]`: what a Claude Code SESSION
 * registered from each corpus plugin, as the spec recorded it (`oracle:record corpus-runtime` in its
 * `extend/tools/`, under `<oracle cache>/<release>/corpus-runtime/`), against what the kit's session
 * registers (`kitRegistry`): the plugin loaded, its agents, slash commands, skills, MCP servers and
 * the output style. The spec asked only the plugins a session runs nothing of on its way to `init`;
 * the rest are counted as skipped. The release defaults to the one `spec/SOURCE.json` pins. Prints
 * the agreement per field and writes every difference to `.cache/corpus/runtime-report.json`.
 */
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { z } from 'zod';
import { kitRegistry } from '../oracle/kit-registry.ts';
import { registrySchema } from '../oracle/recorded.ts';
import { compareRegistries, FIELDS, type FieldVerdict } from '../oracle/registry-diff.ts';
import { readSource } from '../spec/source.ts';
import { FILL, ORACLE_CACHE_DIR, readCorpusTargets, REPORTS_DIR } from './location.ts';
import { pool } from './pool.ts';

const outcomeSchema = z.discriminatedUnion('status', [
  z.object({ status: z.enum(['skipped', 'no-init']) }),
  z.object({
    status: z.literal('recorded'),
    plugins: z.array(z.string()),
    registry: registrySchema,
  }),
]);

type Row =
  | { readonly plugin: string; readonly status: 'skipped' | 'no-init' | 'unrecorded' }
  | {
      readonly plugin: string;
      readonly status: 'compared';
      readonly differs: readonly FieldVerdict[];
    };

const argument = (flag: string): string | undefined => {
  const at = process.argv.indexOf(flag);
  return at === -1 ? undefined : process.argv[at + 1];
};
const release = argument('--release') ?? (await readSource()).release;
const recordedDir = join(ORACLE_CACHE_DIR, release, 'corpus-runtime');

async function compareOne(plugin: string, dir: string): Promise<Row> {
  const file = Bun.file(join(recordedDir, `${plugin}.json`));
  const outcome = outcomeSchema.safeParse(await file.json().catch(() => undefined));
  if (!outcome.success) return { plugin, status: 'unrecorded' };
  if (outcome.data.status !== 'recorded') return { plugin, status: outcome.data.status };
  const verdicts = compareRegistries(plugin, await kitRegistry(dir), outcome.data.registry, {});
  return { plugin, status: 'compared', differs: verdicts.filter((v) => v.status === 'differs') };
}

const limit = Number(argument('--limit') ?? Infinity);
const onlyList = argument('--only');
const only = onlyList === undefined ? undefined : new Set(onlyList.split(','));
const plugins = (await readCorpusTargets())
  .filter(({ key }) => only === undefined || only.has(key))
  .slice(0, limit);

const rows = await pool(plugins, 6, async ({ key, dir }) =>
  compareOne(key, dir).catch((error: unknown): Row => ({
    plugin: `${key}: ${String(error)}`,
    status: 'no-init',
  })),
);

const compared = rows.filter((row) => row.status === 'compared');
const count = (status: Row['status']) => rows.filter((row) => row.status === status).length;
console.log(`Claude Code ${release} session, as the spec recorded it, over ${rows.length} plugins`);
console.log(
  `  compared ${compared.length}, skipped ${count('skipped')} (they run code at start), no init ${count('no-init')}`,
);
if (count('unrecorded') > 0) {
  console.log(
    `  not recorded ${count('unrecorded')}: \`bun run oracle:record corpus-runtime\`, ${FILL}`,
  );
}
const whole = compared.filter((row) => row.differs.length === 0).length;
console.log(
  `  whole registry agrees: ${((100 * whole) / Math.max(compared.length, 1)).toFixed(1)}%`,
);
for (const field of FIELDS) {
  const n = compared.filter((row) => row.differs.some((v) => v.field === field)).length;
  console.log(`  ${field} differs: ${n}`);
}
const report = join(REPORTS_DIR, 'runtime-report.json');
await mkdir(REPORTS_DIR, { recursive: true });
await writeFile(
  report,
  `${JSON.stringify(
    rows.filter((row) =>
      row.status === 'compared' ? row.differs.length > 0 : row.status !== 'skipped',
    ),
    null,
    2,
  )}\n`,
);
console.log(`Report: ${report}`);
