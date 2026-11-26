/**
 * Which Rust files are ports of which TypeScript files, and whether each is current
 * (docs/RUST.md, RS9). A port's first line names its source and the source's hash when it was
 * ported:
 *
 *   //! Port of `src/format/messages.ts` at sha256:0123456789ab.
 *
 * When the TypeScript file changes (a spec revision synced, a fix), the hash no longer matches
 * and the port is stale: the check names it, and the TypeScript diff since that hash is what the
 * port has to take. The vectors carry the same hashes (`tests/vectors/SOURCES.json`, written by
 * `scripts/rust/record.ts`), so vectors recorded from an older source are named too.
 *
 *   bun scripts/rust/ports.ts                 list every port and whether it is current
 *   bun scripts/rust/ports.ts --check         fail on a stale port, stale vectors or a bad header
 *   bun scripts/rust/ports.ts --stamp <rs>…   write the current hash into each port's header
 *   bun scripts/rust/ports.ts --diff <rs>     the source's diff since the revision the port
 *                                             was made from: what the port has to take
 */
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = join(import.meta.dir, '..', '..');
const crate = join(root, 'crates', 'agent-plugins');
const sourcesFile = join(crate, 'tests', 'vectors', 'SOURCES.json');
const HEADER =
  /^\/\/! (?:Port|The core's counterpart) of `(src\/[^`]+\.ts)`(?: at sha256:([0-9a-f]{12}|STAMP)|(, switched))?/u;

/** The first 12 hex digits of a source file's SHA-256: enough to tell two revisions apart. */
export function sourceHash(path: string): string {
  return createHash('sha256')
    .update(readFileSync(join(root, path)))
    .digest('hex')
    .slice(0, 12);
}

interface Port {
  readonly file: string;
  readonly source: string;
  readonly stamped: string | undefined;
  /** The TypeScript is the port's wrapper (docs/RUST.md, "Switching a module"). */
  readonly switched: boolean;
  /** The TypeScript stays (docs/RUST.md, "What stays TypeScript"): only its hash is followed. */
  readonly counterpart: boolean;
}

function rustFiles(): string[] {
  return readdirSync(join(crate, 'src'), { recursive: true, encoding: 'utf8' })
    .filter((name) => name.endsWith('.rs'))
    .map((name) => join(crate, 'src', name))
    .toSorted();
}

function portOf(file: string): Port | undefined {
  const first = readFileSync(file, 'utf8').split('\n', 1)[0] ?? '';
  const match = HEADER.exec(first);
  if (match?.[1] === undefined) return undefined;
  return {
    file: relative(root, file),
    source: match[1],
    stamped: match[2] === 'STAMP' ? undefined : match[2],
    switched: match[3] !== undefined,
    counterpart: first.startsWith("//! The core's counterpart"),
  };
}

interface Recorded {
  readonly spec: string;
  readonly sources: Readonly<Record<string, string>>;
}

function recorded(): Recorded | undefined {
  if (!existsSync(sourcesFile)) return undefined;
  return JSON.parse(readFileSync(sourcesFile, 'utf8')) as Recorded;
}

/** The spec revision the package is synced to (`spec/SOURCE.json`). */
function pinnedSpec(): string {
  return (JSON.parse(readFileSync(join(root, 'spec', 'SOURCE.json'), 'utf8')) as { commit: string })
    .commit;
}

/** The names a Rust module's `exports!` block declares, and its module path. */
function rustExports(file: string): { readonly module?: string; readonly names: string[] } {
  const text = readFileSync(join(root, file), 'utf8');
  const at = text.indexOf('exports! {');
  if (at < 0) return { names: [] };
  const block = text.slice(at);
  const module = /^\s*exports! \{\s*"([^"]+)"/u.exec(block)?.[1];
  const names = [...block.matchAll(/^\s*"([A-Za-z_$][\w$]*)" =>/gmu)].map(
    (match) => match[1] ?? '',
  );
  return module === undefined ? { names } : { module, names };
}

/** What is wrong with a switched module: its wrapper and its Rust exports must name the same. */
function switchedProblems(port: Port): string[] {
  const found: string[] = [];
  const { module, names } = rustExports(port.file);
  const expected = port.source.replace(/^src\//u, '').replace(/\.ts$/u, '');
  if (module !== expected)
    found.push(`${port.file}: exports! names "${module ?? ''}", not "${expected}"`);
  const wrapper = readFileSync(join(root, port.source), 'utf8');
  if (!wrapper.includes(`coreModule('${expected}')`)) {
    found.push(`${port.source}: no coreModule('${expected}')`);
  }
  const called = new Set(
    [
      ...wrapper.matchAll(
        /\bcall(?:Core|\.async)?(?:<[^>]*>)?\(\s*'(?:[^'#]+#)?([A-Za-z_$][\w$]*)'/gu,
      ),
    ].map((match) => match[1] ?? ''),
  );
  for (const name of names) {
    if (!called.has(name)) found.push(`${port.source}: never calls the core's ${name}`);
  }
  for (const name of called) {
    if (!names.includes(name))
      found.push(`${port.source}: calls ${name}, which ${port.file} does not export`);
  }
  const vectorsOf = join(crate, 'tests', 'vectors', `${expected}.jsonl`);
  if (existsSync(vectorsOf))
    found.push(`${relative(root, vectorsOf)}: a switched module keeps no vectors`);
  return found;
}

function problems(): string[] {
  const found: string[] = [];
  const ports = rustFiles().flatMap((file) => portOf(file) ?? []);
  // Vectors hold a ported module until it is switched; with none in between, none are needed
  // (they are a local cache, not committed: docs/RUST.md).
  const held = ports.some(
    (port) => !port.switched && !port.counterpart && existsSync(join(root, port.source)),
  );
  const vectors = recorded();
  if (vectors === undefined) {
    if (held) found.push('tests/vectors/SOURCES.json is missing: bun scripts/rust/record.ts');
  } else if (held && vectors.spec !== pinnedSpec()) {
    found.push(
      `the vectors were recorded at spec ${vectors.spec.slice(0, 12)}, the package is synced to ${pinnedSpec().slice(0, 12)}: bun scripts/rust/record.ts`,
    );
  }
  const hashes = vectors?.sources ?? {};
  for (const port of ports) {
    if (!existsSync(join(root, port.source))) {
      found.push(`${port.file}: its source ${port.source} no longer exists`);
      continue;
    }
    if (port.switched) {
      found.push(...switchedProblems(port));
      continue;
    }
    const now = sourceHash(port.source);
    if (port.stamped === undefined) found.push(`${port.file}: no hash; --stamp it`);
    else if (port.stamped !== now) {
      found.push(`${port.file}: stale, ${port.source} changed since sha256:${port.stamped}`);
    }
    const at = port.counterpart ? undefined : hashes[port.source];
    if (at !== undefined && at !== now) {
      found.push(`${port.source}: vectors recorded at sha256:${at}; record them again`);
    }
  }
  return found;
}

const hashOfBytes = (bytes: Uint8Array): string =>
  createHash('sha256').update(bytes).digest('hex').slice(0, 12);

function git(args: readonly string[]): { readonly ok: boolean; readonly out: Uint8Array } {
  const done = Bun.spawnSync(['git', ...args], { cwd: root, stdout: 'pipe', stderr: 'pipe' });
  return { ok: done.exitCode === 0, out: done.stdout };
}

/** The newest commit whose copy of `source` has the hash `stamped`, if git has one. */
function commitWith(source: string, stamped: string): string | undefined {
  const commits = new TextDecoder().decode(git(['log', '--format=%H', '--', source]).out);
  for (const commit of commits.split('\n').filter((line) => line !== '')) {
    const shown = git(['show', `${commit}:${source}`]);
    if (shown.ok && hashOfBytes(shown.out) === stamped) return commit;
  }
  return undefined;
}

function diff(file: string): void {
  const port = portOf(file);
  if (port?.stamped === undefined) throw new Error(`${file}: no stamped port header`);
  if (port.stamped === sourceHash(port.source)) {
    console.log(`${port.file} is current with ${port.source}`);
    return;
  }
  const commit = commitWith(port.source, port.stamped);
  if (commit === undefined) {
    throw new Error(
      `${port.source} at sha256:${port.stamped} is in no commit: it was ported from uncommitted work`,
    );
  }
  Bun.spawnSync(['git', 'diff', commit, '--', port.source], {
    cwd: root,
    stdout: 'inherit',
    stderr: 'inherit',
  });
}

function stamp(files: readonly string[]): void {
  for (const file of files) {
    const text = readFileSync(file, 'utf8');
    const [first = '', ...rest] = text.split('\n');
    const match = HEADER.exec(first);
    if (match?.[1] === undefined) throw new Error(`${file}: the first line names no source`);
    const hash = `at sha256:${sourceHash(match[1])}`;
    // Only the hash changes: a counterpart keeps its words, a port without one gains it.
    const line = / at sha256:(?:[0-9a-f]{12}|STAMP)/u.test(first)
      ? first.replace(/at sha256:(?:[0-9a-f]{12}|STAMP)/u, hash)
      : first.replace(/`(src\/[^`]+\.ts)`\.?/u, `\`$1\` ${hash}.`);
    writeFileSync(file, [line, ...rest].join('\n'));
    console.log(line);
  }
}

function list(): void {
  for (const port of rustFiles().flatMap((file) => portOf(file) ?? [])) {
    const current = existsSync(join(root, port.source)) && port.stamped === sourceHash(port.source);
    console.log(`${current ? 'current' : 'STALE  '}  ${port.file}  ←  ${port.source}`);
  }
}

if (import.meta.main) {
  const [mode, ...files] = process.argv.slice(2);
  if (mode === '--stamp') stamp(files);
  else if (mode === '--diff') for (const file of files) diff(file);
  else if (mode === '--check') {
    const found = problems();
    for (const line of found) console.error(line);
    if (found.length > 0) process.exitCode = 1;
  } else list();
}
