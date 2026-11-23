/**
 * Agent Plugins Extended §4.8.5 item 3, "Bare repository folders" (skills.md, "Permission checks on
 * injected commands"): a folder looks bare when it holds a `HEAD` file directly beside `refs` or
 * `commondir`; when the closest repository at or above the working directory looks bare, a Bash
 * command that contains `git` is not covered by `allowed-tools`. The session finds that repository
 * itself through `HostPorts.folders`: the working directory, then each parent up to the top of the
 * repository; a folder holding `.git` is a working tree, not bare. `ExpandOptions.bareRepository`
 * given by the host decides instead.
 */
import { expect, test } from 'bun:test';
import { parseRule } from '../../src/rules/index.ts';
import { loaded, signal } from '../support/host.ts';
import { folderHostOf, PROJECT } from '../support/scan.ts';
import { spec } from '../support/spec.ts';

const json = (value: unknown): string => JSON.stringify(value);

const BARE = { [`${PROJECT}/HEAD`]: 'ref: refs/heads/main\n', [`${PROJECT}/refs/heads/main`]: 'x' };

async function expandIn(
  files: Record<string, string>,
  options: { readonly repositories?: readonly string[]; readonly bareRepository?: boolean } = {},
) {
  const plugin = await loaded({
    '.claude-plugin/plugin.json': json({ name: 'ops' }),
    'skills/branch/SKILL.md':
      '---\ndescription: Show the branch\nallowed-tools: Bash(git *)\n---\nBranch: !`git rev-parse --abbrev-ref HEAD`',
  });
  const { session, runner } = folderHostOf(files, {
    plugins: [{ id: 'ops@acme', plugin }],
    runs: [{ stdout: 'main\n' }],
    ...(options.repositories === undefined ? {} : { disk: { repositories: options.repositories } }),
  });
  const expansion = await session.expandSkill('ops:branch', {
    signal: signal(),
    ...(options.bareRepository === undefined ? {} : { bareRepository: options.bareRepository }),
  });
  return { expansion, runner };
}

const abortOf = (expansion: { readonly ok: boolean; readonly issues?: readonly unknown[] }) =>
  JSON.stringify(expansion);

spec('skills.behavior.Permission checks on injected commands.Bare repository folders', () => {
  test('a working directory that looks bare: the git command is not covered by allowed-tools', async () => {
    const { expansion, runner } = await expandIn(BARE);
    expect(expansion.ok).toBe(false);
    expect(abortOf(expansion)).toContain('Shell command permission check failed');
    expect(runner.calls).toHaveLength(0);
  });

  test('`commondir` beside `HEAD` looks bare too', async () => {
    const files = { [`${PROJECT}/HEAD`]: 'ref', [`${PROJECT}/commondir`]: '../..' };
    expect((await expandIn(files)).expansion.ok).toBe(false);
  });

  test('a parent folder of the working directory, inside the repository, that looks bare counts', async () => {
    const files = { '/work/HEAD': 'ref', '/work/refs/heads/main': 'x', [`${PROJECT}/a.txt`]: '' };
    const { expansion } = await expandIn(files, { repositories: ['/work'] });
    expect(expansion.ok).toBe(false);
  });

  test('a working tree (a folder with `.git`) is the closest repository: the grant holds', async () => {
    const files = {
      '/work/HEAD': 'ref',
      '/work/refs/heads/main': 'x',
      [`${PROJECT}/.git/HEAD`]: 'ref',
    };
    const { expansion, runner } = await expandIn(files, { repositories: ['/work'] });
    expect(expansion.ok).toBe(true);
    expect(runner.calls).toHaveLength(1);
  });

  test('a folder with only `HEAD`, or no repository at all: the grant holds', async () => {
    expect((await expandIn({ [`${PROJECT}/HEAD`]: 'ref' })).expansion.ok).toBe(true);
    expect((await expandIn({ [`${PROJECT}/a.txt`]: '' })).expansion.ok).toBe(true);
  });

  test("the host's own answer decides instead", async () => {
    expect((await expandIn(BARE, { bareRepository: false })).expansion.ok).toBe(true);
    const normal = { [`${PROJECT}/a.txt`]: '' };
    expect((await expandIn(normal, { bareRepository: true })).expansion.ok).toBe(false);
  });

  test('a permission rule that allows the command lets it run in a bare-looking folder', async () => {
    const plugin = await loaded({
      '.claude-plugin/plugin.json': json({ name: 'ops' }),
      'skills/branch/SKILL.md':
        '---\ndescription: Show the branch\n---\nBranch: !`git rev-parse --abbrev-ref HEAD`',
    });
    const allow = parseRule('Bash(git rev-parse *)', {
      behavior: 'allow',
      source: { kind: 'user' },
    }).rule;
    const { session } = folderHostOf(BARE, {
      plugins: [{ id: 'ops@acme', plugin }],
      runs: [{ stdout: 'main\n' }],
      session: { permissionRules: allow === undefined ? [] : [allow] },
    });
    expect((await session.expandSkill('ops:branch', { signal: signal() })).ok).toBe(true);
  });
});
