/**
 * `bun run spec:sync [ref] [--remote]`: copies the spec files (`SYNCED`) and folders
 * (`SYNCED_FOLDERS`, replaced whole) from Agent Plugins Extended at a git ref into `spec/` and
 * rewrites `spec/SOURCE.json`. The ref defaults to the commit
 * `SOURCE.json` pins, so a plain run restores the pinned files; `main` or a tag moves the pin.
 *
 * The files are read from the checkout beside this repository (`../agent-plugins-extended`) when
 * it has the ref, else from the GitHub repository: `git` over HTTPS, then `gh`. `--remote` skips the
 * checkout. Every file must name the same Claude Code release; a ref whose files disagree is refused
 * and nothing is written.
 */
import { existsSync } from 'node:fs';
import { mkdir, mkdtemp, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import {
  SIBLING_CHECKOUT,
  SOURCE_FILE,
  SPEC_DIR,
  SPEC_FOLDER,
  SPEC_REPOSITORY,
  SPEC_SLUG,
  NAMING_RELEASE,
  SYNCED,
  SYNCED_FOLDERS,
  hashOf,
  readSource,
  releaseIn,
  type Source,
} from './source.ts';

/** One file of a synced folder: its path below `spec/`, whether it is a symbolic link, its bytes. */
interface Entry {
  readonly name: string;
  readonly link: boolean;
  readonly read: () => Promise<Uint8Array>;
}

/**
 * The spec at one commit: where it is read from, a reader of a file by its path there, and the
 * files under a folder there.
 */
interface Origin {
  readonly label: string;
  readonly commit: string;
  readonly read: (path: string) => Promise<Uint8Array>;
  readonly list: (folder: string) => Promise<Entry[]>;
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

/** `<mode> <sha> <path>` lines of a tree listing as entries below `folder`; mode 120000 is a link. */
function entriesOf(
  listing: string | undefined,
  folder: string,
  reader: (path: string, sha: string) => () => Promise<Uint8Array>,
): Entry[] {
  return (listing ?? '').split('\n').flatMap((line) => {
    const match = /^(\d+) ([0-9a-f]+) (.+)$/.exec(line);
    const [, mode, sha = '', path = ''] = match ?? [];
    if (mode === undefined || !path.startsWith(`${folder}/`)) return [];
    const name = path.slice(SPEC_FOLDER.length + 1);
    return [{ name, link: mode === '120000', read: reader(path, sha) }];
  });
}

/** An origin a git repository answers: the checkout beside, or a shallow fetch. */
function gitOrigin(git: readonly string[], commit: string, label: string): Origin {
  const show = (path: string) =>
    must(run([...git, 'show', `${commit}:${path}`]), `${path} at ${commit}`);
  const list = async (folder: string) => {
    const format = '--format=%(objectmode) %(objectname) %(path)';
    const listing = text(await run([...git, 'ls-tree', '-r', format, commit, '--', folder]));
    return entriesOf(listing, folder, (path) => () => show(path));
  };
  return { label, commit, read: show, list };
}

async function fromCheckout(ref: string): Promise<Origin | undefined> {
  if (!existsSync(join(SIBLING_CHECKOUT, '.git'))) return undefined;
  const git = ['git', '-C', SIBLING_CHECKOUT];
  const commit = text(await run([...git, 'rev-parse', '--verify', '--quiet', `${ref}^{commit}`]));
  if (commit === undefined) return undefined;
  return gitOrigin(git, commit, SIBLING_CHECKOUT);
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
  return gitOrigin(git, commit, `${SPEC_REPOSITORY} (git)`);
}

async function fromGh(ref: string): Promise<Origin | undefined> {
  const api = ['gh', 'api', '-H', 'Accept: application/vnd.github.raw'];
  const commit = text(
    await run(['gh', 'api', `repos/${SPEC_SLUG}/commits/${ref}`, '--jq', '.sha']),
  );
  if (commit === undefined) return undefined;
  const tree = `repos/${SPEC_SLUG}/git/trees/${commit}?recursive=1`;
  const jq = '.tree[] | select(.type == "blob") | "\\(.mode) \\(.sha) \\(.path)"';
  return {
    label: `${SPEC_REPOSITORY} (gh)`,
    commit,
    read: (path) =>
      must(run([...api, `repos/${SPEC_SLUG}/contents/${path}?ref=${commit}`]), `${path} via gh`),
    list: async (folder) =>
      entriesOf(
        text(await run(['gh', 'api', tree, '--jq', jq])),
        folder,
        (path, sha) => () =>
          must(run([...api, `repos/${SPEC_SLUG}/git/blobs/${sha}`]), `${path} via gh`),
      ),
  };
}

/** Writes one synced file, a symbolic link as a link to the path its bytes name. */
async function writeEntry(name: string, link: boolean, bytes: Uint8Array): Promise<void> {
  const path = join(SPEC_DIR, name);
  await mkdir(dirname(path), { recursive: true });
  if (link) await symlink(new TextDecoder().decode(bytes), path);
  else await Bun.write(path, bytes);
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
  const files: { name: string; link: boolean; bytes: Uint8Array }[] = await Promise.all(
    SYNCED.map(async (name) => ({
      name,
      link: false,
      bytes: await origin.read(`${SPEC_FOLDER}/${name}`),
    })),
  );
  for (const folder of SYNCED_FOLDERS) {
    const entries = await origin.list(`${SPEC_FOLDER}/${folder}`);
    if (entries.length === 0)
      throw new Error(`${SPEC_FOLDER}/${folder} is empty at ${origin.commit}`);
    for (const { name, link, read } of entries) files.push({ name, link, bytes: await read() });
  }
  const releases = new Set(
    files
      .filter(({ name }) => NAMING_RELEASE.some((one) => one === name))
      .map(({ bytes }) => releaseIn(bytes) ?? 'none'),
  );
  const [release, ...others] = releases;
  if (release === undefined || release === 'none' || others.length > 0) {
    throw new Error(`the snapshots at ${origin.commit} name releases ${[...releases].join(', ')}`);
  }
  for (const folder of SYNCED_FOLDERS)
    await rm(join(SPEC_DIR, folder), { recursive: true, force: true });
  for (const { name, link, bytes } of files) await writeEntry(name, link, bytes);
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
