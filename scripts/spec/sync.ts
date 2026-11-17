/**
 * `bun run spec:sync [ref] [--remote]`: copies the spec files (`SYNCED`) from Agent Plugins Extended
 * at a git ref into `spec/` and rewrites `spec/SOURCE.json`. The ref defaults to the commit
 * `SOURCE.json` pins, so a plain run restores the pinned files; `main` or a tag moves the pin.
 *
 * The files are read from the checkout beside this repository (`../agent-plugins-extended`) when
 * it has the ref, else from the GitHub repository: `git` over HTTPS, then `gh`. `--remote` skips the
 * checkout. Every file must name the same Claude Code release; a ref whose files disagree is refused
 * and nothing is written.
 */
import { existsSync } from 'node:fs';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  SIBLING_CHECKOUT,
  SOURCE_FILE,
  SPEC_DIR,
  SPEC_FOLDER,
  SPEC_REPOSITORY,
  SPEC_SLUG,
  NAMING_RELEASE,
  SYNCED,
  hashOf,
  readSource,
  releaseIn,
  type Source,
} from './source.ts';

/** The spec at one commit: where it is read from, and a reader of a file by its path there. */
interface Origin {
  readonly label: string;
  readonly commit: string;
  readonly read: (path: string) => Promise<Uint8Array>;
}

/** Runs a program; its stdout, or undefined when it fails. */
async function run(command: readonly string[]): Promise<Uint8Array | undefined> {
  try {
    const child = Bun.spawn([...command], { stdout: 'pipe', stderr: 'pipe' });
    const [out] = await Promise.all([new Response(child.stdout).arrayBuffer(), child.exited]);
    return child.exitCode === 0 ? new Uint8Array(out) : undefined;
  } catch {
    return undefined;
  }
}

const text = (bytes: Uint8Array | undefined): string | undefined =>
  bytes === undefined ? undefined : new TextDecoder().decode(bytes).trim();

const must = async (bytes: Promise<Uint8Array | undefined>, what: string): Promise<Uint8Array> =>
  (await bytes) ?? Promise.reject(new Error(`could not read ${what}`));

async function fromCheckout(ref: string): Promise<Origin | undefined> {
  if (!existsSync(join(SIBLING_CHECKOUT, '.git'))) return undefined;
  const git = ['git', '-C', SIBLING_CHECKOUT];
  const commit = text(await run([...git, 'rev-parse', '--verify', '--quiet', `${ref}^{commit}`]));
  if (commit === undefined) return undefined;
  return {
    label: SIBLING_CHECKOUT,
    commit,
    read: (path) => must(run([...git, 'show', `${commit}:${path}`]), `${path} at ${commit}`),
  };
}

async function fromGit(ref: string, scratch: string): Promise<Origin | undefined> {
  const git = ['git', '-C', scratch];
  const steps = [
    ['git', 'init', '-q', scratch],
    [...git, 'remote', 'add', 'origin', `${SPEC_REPOSITORY}.git`],
    [...git, 'fetch', '-q', '--depth', '1', 'origin', ref],
  ];
  for (const step of steps) if ((await run(step)) === undefined) return undefined;
  const commit = text(await run([...git, 'rev-parse', 'FETCH_HEAD']));
  if (commit === undefined) return undefined;
  return {
    label: `${SPEC_REPOSITORY} (git)`,
    commit,
    read: (path) => must(run([...git, 'show', `${commit}:${path}`]), `${path} at ${commit}`),
  };
}

async function fromGh(ref: string): Promise<Origin | undefined> {
  const api = ['gh', 'api', '-H', 'Accept: application/vnd.github.raw'];
  const commit = text(
    await run(['gh', 'api', `repos/${SPEC_SLUG}/commits/${ref}`, '--jq', '.sha']),
  );
  if (commit === undefined) return undefined;
  return {
    label: `${SPEC_REPOSITORY} (gh)`,
    commit,
    read: (path) =>
      must(run([...api, `repos/${SPEC_SLUG}/contents/${path}?ref=${commit}`]), `${path} via gh`),
  };
}

async function pinnedCommit(): Promise<string> {
  const pinned = await readSource().catch(() => undefined);
  if (pinned === undefined) throw new Error('spec/SOURCE.json pins no commit: name a ref');
  return pinned.commit;
}

const args = process.argv.slice(2);
const ref = args.find((arg) => !arg.startsWith('--')) ?? (await pinnedCommit());
const scratch = await mkdtemp(join(tmpdir(), 'agent-plugins-spec-'));
try {
  const origin =
    (args.includes('--remote') ? undefined : await fromCheckout(ref)) ??
    (await fromGit(ref, scratch)) ??
    (await fromGh(ref));
  if (origin === undefined) throw new Error(`${ref} was found neither beside nor on GitHub`);
  const files = await Promise.all(
    SYNCED.map(async (name) => ({ name, bytes: await origin.read(`${SPEC_FOLDER}/${name}`) })),
  );
  const releases = new Set(
    files
      .filter(({ name }) => NAMING_RELEASE.some((one) => one === name))
      .map(({ bytes }) => releaseIn(bytes) ?? 'none'),
  );
  const [release, ...others] = releases;
  if (release === undefined || release === 'none' || others.length > 0) {
    throw new Error(`the snapshots at ${origin.commit} name releases ${[...releases].join(', ')}`);
  }
  for (const { name, bytes } of files) await Bun.write(join(SPEC_DIR, name), bytes);
  const source: Source = {
    repository: SPEC_REPOSITORY,
    commit: origin.commit,
    release,
    files: Object.fromEntries(files.map(({ name, bytes }) => [name, hashOf(bytes)])),
  };
  await Bun.write(SOURCE_FILE, `${JSON.stringify(source, null, 2)}\n`);
  console.log(
    `spec/ is Agent Plugins Extended ${origin.commit} (${origin.label}), Claude Code ${release}`,
  );
} finally {
  await rm(scratch, { recursive: true, force: true });
}
