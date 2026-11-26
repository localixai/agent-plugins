/**
 * Records the vectors the Rust crate is held to: runs `bun test` over the given test paths (all
 * of `tests/` by default) with the recorder preloaded (scripts/rust/recorder/), then writes one
 * file per `src/` module to `crates/agent-plugins/tests/vectors/<module>.jsonl`: every distinct
 * call of an exported function whose arguments are plain data, with what it returned or threw and
 * the requirement ids the calling tests ran under. Lines are sorted, so a re-recording diffs
 * cleanly. A module whose functions were called with plain data in none of the tests gets no file.
 * With no paths every file is written anew; with paths, what they record is added to the files
 * (a call recorded again replaces its line).
 *
 *   bun scripts/rust/record.ts [tests/<path>…]
 */
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { sourceHash } from './ports.ts';

interface Line {
  readonly module: string;
  readonly fn: string;
  readonly args: unknown;
  readonly async: boolean;
  readonly outcomes: readonly unknown[];
  readonly spec: readonly string[];
}

const root = join(import.meta.dir, '..', '..');
const vectors = join(root, 'crates', 'agent-plugins', 'tests', 'vectors');
const sourcesFile = join(vectors, 'SOURCES.json');
const preload = join(import.meta.dir, 'recorder', 'preload.ts');

/**
 * Every call through the recorder costs more, and adds stack frames: a test that recurses
 * through tens of thousands of exported calls (a 28,000-level tree) runs past bun's default
 * 5-second limit under it, so the recording allows each test a minute.
 */
const TEST_TIMEOUT_MS = '60000';

function runTests(paths: readonly string[], out: string): number {
  const run = Bun.spawnSync(
    ['bun', 'test', '--timeout', TEST_TIMEOUT_MS, '--preload', preload, ...paths],
    {
      cwd: root,
      env: { ...process.env, RECORD_OUT: out },
      stdout: 'inherit',
      stderr: 'inherit',
    },
  );
  return run.exitCode;
}

function merge(out: string): Map<string, Map<string, Line>> {
  const modules = new Map<string, Map<string, Line>>();
  for (const name of readdirSync(out)) {
    for (const text of readFileSync(join(out, name), 'utf8').split('\n')) {
      if (text === '') continue;
      const line = JSON.parse(text) as Line;
      const calls = modules.get(line.module) ?? new Map<string, Line>();
      modules.set(line.module, calls);
      const key = keyOf(line.fn, line.args);
      const known = calls.get(key);
      const spec = new Set([...(known?.spec ?? []), ...line.spec]);
      calls.set(key, { ...line, spec: [...spec].toSorted() });
    }
  }
  return modules;
}

const keyOf = (fn: string, args: unknown): string => `${fn}\u0000${JSON.stringify(args)}`;

/** The calls a module's file holds already: a recording of some test files adds to them. */
function known(file: string, module: string): Map<string, Line> {
  const calls = new Map<string, Line>();
  if (!existsSync(file)) return calls;
  for (const text of readFileSync(file, 'utf8').split('\n')) {
    if (text === '') continue;
    const read = JSON.parse(text) as Omit<Line, 'module' | 'async'> & { readonly async?: boolean };
    const line: Line = { ...read, module, async: read.async ?? false };
    calls.set(keyOf(line.fn, line.args), line);
  }
  return calls;
}

function write(modules: Map<string, Map<string, Line>>, replaceAll: boolean): number {
  if (replaceAll) rmSync(vectors, { recursive: true, force: true });
  let count = 0;
  for (const [module, recorded] of modules) {
    const file = join(vectors, `${module}.jsonl`);
    mkdirSync(dirname(file), { recursive: true });
    const calls = replaceAll ? recorded : new Map([...known(file, module), ...recorded]);
    const lines = [...calls.entries()]
      .toSorted(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
      .map(([, line]) =>
        JSON.stringify({
          fn: line.fn,
          args: line.args,
          ...(line.async ? { async: true } : {}),
          outcomes: line.outcomes,
          spec: line.spec,
        }),
      );
    count += lines.length;
    writeFileSync(file, lines.join('\n') + '\n');
  }
  return count;
}

interface Sources {
  readonly spec: string;
  readonly sources: Readonly<Record<string, string>>;
}

/**
 * What the vectors were recorded from: the spec revision `spec/SOURCE.json` pins and the hash of
 * each `src/` module they hold (scripts/rust/ports.ts reads it, RS9). A partial recording keeps
 * the hashes of the modules it did not touch.
 */
function writeSources(modules: ReadonlyMap<string, unknown>, replaceAll: boolean): void {
  const pin = JSON.parse(readFileSync(join(root, 'spec', 'SOURCE.json'), 'utf8')) as {
    readonly commit: string;
  };
  const spec = pin.commit;
  const before: Sources =
    replaceAll || !existsSync(sourcesFile)
      ? { spec, sources: {} }
      : (JSON.parse(readFileSync(sourcesFile, 'utf8')) as Sources);
  const sources: Record<string, string> = { ...before.sources };
  for (const module of modules.keys()) sources[`src/${module}.ts`] = sourceHash(`src/${module}.ts`);
  const sorted = Object.fromEntries(
    Object.entries(sources).toSorted(([a], [b]) => (a < b ? -1 : 1)),
  );
  writeFileSync(sourcesFile, `${JSON.stringify({ spec, sources: sorted }, null, 2)}\n`);
}

const paths = process.argv.slice(2);
const out = mkdtempSync(join(tmpdir(), 'agent-plugins-record-'));
const exit = runTests(paths.length > 0 ? paths : ['tests'], out);
const modules = merge(out);
const count = write(modules, paths.length === 0);
writeSources(modules, paths.length === 0);
rmSync(out, { recursive: true, force: true });
console.log(`\nrecorded ${count} calls in ${modules.size} modules (bun test exited ${exit})`);
process.exitCode = exit;
