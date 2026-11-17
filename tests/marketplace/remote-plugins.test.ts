/**
 * desktop.md § Install plugins ("Plugins aren't available in WSL sessions"), § Windows remote
 * machines (a plugin holding a name Windows can't store, such as one with `:`, is not copied and
 * the session lists it), § Files and processes on the remote machine (`~/.claude/remote/plugins/`).
 * No requirement id covers these paragraphs; what else Windows can't store is a judgment
 * (spec/judgments.md), so the tests stand outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import {
  planRemotePlugins,
  REMOTE_PLUGINS_DIRECTORY,
} from '../../src/marketplace/remote-plugins.ts';

const plain = { id: 'fmt@team', paths: ['.claude-plugin/plugin.json', 'skills/fmt/SKILL.md'] };
const colon = { id: 'clock@team', paths: ['skills/at:noon/SKILL.md'] };

describe('plugins on a remote machine', () => {
  test('an SSH session on Linux or macOS gets every plugin copied', () => {
    expect(REMOTE_PLUGINS_DIRECTORY).toBe('~/.claude/remote/plugins/');
    expect(planRemotePlugins('posix', [plain, colon])).toEqual({
      copy: ['fmt@team', 'clock@team'],
      leftOut: [],
    });
  });

  test('a Windows remote machine leaves out a plugin holding a name with a colon', () => {
    expect(planRemotePlugins('windows', [plain, colon])).toEqual({
      copy: ['fmt@team'],
      leftOut: ['clock@team'],
    });
  });

  test('a WSL session gets no plugins', () => {
    expect(planRemotePlugins('wsl', [plain])).toEqual({ copy: [], leftOut: [] });
  });
});

describe('names Windows cannot store (judgment)', () => {
  test.each([['a?b'], ['CON'], ['nul.txt'], ['trailing.'], ['trailing '], ['a|b'], ['a\u0001']])(
    '%p is left out',
    (name) => {
      const plan = planRemotePlugins('windows', [
        { id: 'x@m', paths: [`skills/${name}/SKILL.md`] },
      ]);
      expect(plan.leftOut).toEqual(['x@m']);
    },
  );

  test('ordinary names, dots inside and a device-like prefix are kept', () => {
    const plan = planRemotePlugins('windows', [
      { id: 'x@m', paths: ['skills/console/SKILL.md', 'bin/run.sh', 'a.b.c/d'] },
    ]);
    expect(plan.copy).toEqual(['x@m']);
  });
});
