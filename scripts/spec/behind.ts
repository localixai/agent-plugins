/**
 * `bun run spec:behind`: whether the spec has moved past the revision `spec/SOURCE.json` pins.
 * It asks GitHub for the spec's `main` (`git ls-remote`) and, when it differs from the pin, lists
 * the commits since the pin that touch `extend/` (from the checkout beside this repository when it
 * has them, else from `gh`). It exits 1 when the pin is behind: being behind is a failure to act
 * on, not a note (AGENTS.md, E7). It is the one check that needs the network, so it is not part of
 * `bun run check`; CI runs it on a schedule (`.github/workflows/spec-behind.yml`).
 *
 * To catch up: `bun run spec:sync main`, then the TypeScript, then the Rust crate (docs/RUST.md,
 * "When the spec moves").
 */
import { readSource, SIBLING_CHECKOUT, SPEC_REPOSITORY, SPEC_SLUG } from './source.ts';

function run(command: readonly string[], cwd: string = process.cwd()): string | undefined {
  const done = Bun.spawnSync([...command], { cwd, stdout: 'pipe', stderr: 'pipe' });
  return done.exitCode === 0 ? done.stdout.toString().trim() : undefined;
}

function latestMain(): string {
  const line = run(['git', 'ls-remote', SPEC_REPOSITORY, 'refs/heads/main']);
  const commit = line?.split(/\s/u)[0];
  if (commit === undefined || commit === '')
    throw new Error(`cannot read main of ${SPEC_REPOSITORY}`);
  return commit;
}

/** The commits from `pin` to `head` that change `extend/`, one line each. */
function commitsSince(pin: string, head: string): readonly string[] {
  run(['git', 'fetch', '--quiet', 'origin', 'main'], SIBLING_CHECKOUT);
  const local = run(
    ['git', 'log', '--oneline', `${pin}..${head}`, '--', 'extend/'],
    SIBLING_CHECKOUT,
  );
  if (local !== undefined) return local === '' ? [] : local.split('\n');
  const compared = run([
    'gh',
    'api',
    `repos/${SPEC_SLUG}/compare/${pin}...${head}`,
    '--jq',
    '.commits[] | "\\(.sha[0:9]) \\(.commit.message | split("\\n")[0])"',
  ]);
  return compared === undefined || compared === '' ? [] : compared.split('\n');
}

const source = await readSource();
const head = latestMain();
if (head === source.commit) {
  console.log(`spec: current (${source.commit.slice(0, 12)}, Claude Code ${source.release})`);
} else {
  const commits = commitsSince(source.commit, head);
  console.error(
    `spec: the pin ${source.commit.slice(0, 12)} (Claude Code ${source.release}) is behind main ${head.slice(0, 12)}`,
  );
  for (const line of commits) console.error(`  ${line}`);
  console.error('catch up: bun run spec:sync main (AGENTS.md, E7)');
  process.exitCode = 1;
}
