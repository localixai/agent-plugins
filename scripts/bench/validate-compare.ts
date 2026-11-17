/**
 * `bun run bench:validate [--release x.y.z]`: `claude plugin validate <dir>` against the kit's
 * `validatePlugin` on the corpus plugins the spec timed (`oracle:record validate-timings` in its
 * `extend/tools/`, the first 30 by `marketplace/name`, cached under
 * `<oracle cache>/<release>/validate-timings.json`; the release defaults to the pinned one). This
 * script never runs `claude`: its column is the spec's record, made on this machine. The kit's
 * columns are run `REPS` times and the median is kept. Columns:
 *
 * - `claude`: one `claude plugin validate <dir>` process, start to exit, as the spec timed it;
 * - `kit process`: one `bun` process that imports the kit and validates the folder (this script with
 *   `--one <dir>`), the same unit of work as the `claude` column;
 * - `kit in process`: `validatePlugin` alone in a warm process, the cost a host pays per plugin.
 *
 * Both programs also validate an empty folder: the floor of every run, the cost of starting the
 * process with nothing to read. The plugins' findings are not compared here (that is `bun run
 * corpus`).
 */
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { validatePlugin } from '../../src/index.ts';
import { diskFiles } from '../corpus/disk-files.ts';
import { z } from 'zod';
import { FILL, ORACLE_CACHE_DIR, readCorpusTargets } from '../corpus/location.ts';
import { readSource } from '../spec/source.ts';

const REPS = 5;

/** What the spec timed: the median ms of `claude plugin validate`, per plugin and on an empty folder. */
const timingsSchema = z.object({
  claudeCodeVersion: z.string(),
  floor: z.number(),
  plugins: z.record(z.string(), z.number()),
});

const median = (samples: readonly number[]): number => {
  const sorted = samples.toSorted((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  const value =
    sorted.length % 2 === 1
      ? sorted[middle]
      : ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2;
  return Math.round((value ?? 0) * 10) / 10;
};

const timeOf = async (work: () => Promise<unknown>): Promise<number> => {
  const start = performance.now();
  await work();
  return performance.now() - start;
};

async function spawned(command: readonly string[]): Promise<number> {
  return timeOf(async () => {
    const run = Bun.spawn([...command], { stdout: 'ignore', stderr: 'ignore' });
    await run.exited;
  });
}

async function medianOf(work: () => Promise<number>): Promise<number> {
  const samples: number[] = [];
  for (let rep = 0; rep < REPS; rep++) samples.push(await work());
  return median(samples);
}

/** The child of the `kit process` column: validate one folder and exit. */
async function one(dir: string): Promise<void> {
  const { issues } = await validatePlugin({ files: diskFiles(dir) });
  process.stdout.write(`${issues.length} issues\n`);
}

interface Row {
  readonly plugin: string;
  readonly claude: number;
  readonly 'kit process': number;
  readonly 'kit in process': number;
}

/** The medians of one folder in the three columns, `claude` the spec's. */
const rowOf = async (plugin: string, dir: string, claude: number): Promise<Row> => ({
  plugin,
  claude,
  'kit process': await medianOf(() => spawned([process.execPath, import.meta.path, '--one', dir])),
  'kit in process': await medianOf(() => timeOf(() => validatePlugin({ files: diskFiles(dir) }))),
});

async function compare(): Promise<void> {
  const at = process.argv.indexOf('--release');
  const release = (at === -1 ? undefined : process.argv[at + 1]) ?? (await readSource()).release;
  const file = join(ORACLE_CACHE_DIR, release, 'validate-timings.json');
  const timed = timingsSchema.safeParse(
    await Bun.file(file)
      .json()
      .catch(() => undefined),
  );
  if (!timed.success) {
    throw new Error(`${file} is missing: \`bun run oracle:record validate-timings\`, ${FILL}`);
  }
  const version = timed.data.claudeCodeVersion;
  const targets = (await readCorpusTargets()).filter(({ key }) => key in timed.data.plugins);
  const empty = await mkdtemp(join(tmpdir(), 'bench-validate-'));
  let floor: Row;
  try {
    floor = await rowOf('(an empty folder: the floor)', empty, timed.data.floor);
  } finally {
    await rm(empty, { recursive: true });
  }
  const rows: Row[] = [];
  for (const target of targets) {
    rows.push(await rowOf(target.key, target.dir, timed.data.plugins[target.key] ?? 0));
  }
  const total = (column: Exclude<keyof Row, 'plugin'>): number =>
    Math.round(rows.reduce((sum, row) => sum + row[column], 0));
  console.log(
    `claude plugin validate (Claude Code ${version}, the spec's timing) vs validatePlugin, ${targets.length} plugins, median of ${REPS} runs, ms`,
  );
  console.table([floor, ...rows]);
  console.table([
    {
      plugins: targets.length,
      claude: total('claude'),
      'kit process': total('kit process'),
      'kit in process': total('kit in process'),
    },
  ]);
}

const dir = process.argv[process.argv.indexOf('--one') + 1];
if (process.argv.includes('--one') && dir !== undefined) await one(dir);
else await compare();
