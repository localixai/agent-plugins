/**
 * errors.md § Path could not be checked: when the client asks whether a plugin path exists and gets
 * an error other than "not found", it does not load what the path names and reports
 * `<component> path couldn't be checked: <path> (<code>)`; the plugin's other components load. The
 * error reaches the kit through `FileStat.checkFailed` (docs/API.md, "Ports").
 */
import { expect, test } from 'bun:test';
import { loadPlugin } from '../../src/loading/index.ts';
import { memoryFiles } from '../support/ports.ts';
import { json, messagesOf } from '../support/plugin-folder.ts';
import { spec } from '../support/spec.ts';

const MANIFEST = json({ name: 'p' });

spec(['cli.errors.Path could not be checked', 'cli.errors.Path could not be checked.ELOOP'], () => {
  test('a default location that loops is not loaded and is reported; the other components load', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': MANIFEST,
      skills: { checkFailed: 'ELOOP' },
      'commands/hi.md': 'Hi.',
    });
    const { plugin, issues } = await loadPlugin({ files });
    expect(plugin?.skills).toEqual([]);
    expect(plugin?.commands).toHaveLength(1);
    expect(messagesOf(issues)).toEqual(["skills path couldn't be checked: skills (ELOOP)"]);
  });

  test('a path that simply does not exist draws no error', async () => {
    const files = memoryFiles({ '.claude-plugin/plugin.json': MANIFEST });
    expect((await loadPlugin({ files })).issues).toEqual([]);
  });
});

spec('cli.errors.Path could not be checked.EIO', () => {
  test('a broken mount under the default monitors file', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': MANIFEST,
      'monitors/monitors.json': { checkFailed: 'EIO' },
    });
    const { plugin, issues } = await loadPlugin({ files });
    expect(plugin?.monitors).toEqual([]);
    expect(messagesOf(issues)).toEqual([
      "monitors path couldn't be checked: monitors/monitors.json (EIO)",
    ]);
  });
});

spec('cli.errors.Path could not be checked.EACCES', () => {
  test('a folder above a declared path that denies traversal refuses the path', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': json({ name: 'p', commands: ['./locked/deploy.md'] }),
      locked: { checkFailed: 'EACCES' },
    });
    const { plugin, issues } = await loadPlugin({ files });
    expect(plugin?.commands).toEqual([]);
    expect(messagesOf(issues)).toEqual([
      "commands path couldn't be checked: ./locked/deploy.md (EACCES)",
    ]);
  });
});

spec('cli.errors.Path could not be checked', () => {
  test('a root SKILL.md that cannot be checked is reported as skills; the other components load', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': MANIFEST,
      'SKILL.md': { checkFailed: 'EIO' },
      'commands/hi.md': 'Hi.',
    });
    const { plugin, issues } = await loadPlugin({ files });
    expect(plugin?.skills).toEqual([]);
    expect(plugin?.commands).toHaveLength(1);
    expect(messagesOf(issues)).toEqual(["skills path couldn't be checked: SKILL.md (EIO)"]);
  });

  test('the plugin directory itself that cannot be checked: nothing of the plugin loads', async () => {
    const files = memoryFiles({ '.': { checkFailed: 'EACCES' }, 'commands/hi.md': 'Hi.' });
    const { plugin, issues } = await loadPlugin({ files, fallbackName: 'p' });
    expect(plugin).toBeUndefined();
    expect(messagesOf(issues)).toEqual(["plugin path couldn't be checked: . (EACCES)"]);
    expect(files.reads).toEqual([]);
  });
});

spec('cli.errors.Plugin not loaded: its manifest was not read', () => {
  test('a manifest that cannot be checked: nothing loads, and the reason ends in the code', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': { checkFailed: 'ELOOP' },
      'commands/hi.md': 'Hi.',
    });
    const { plugin, issues } = await loadPlugin({ files, fallbackName: 'p' });
    expect(plugin).toBeUndefined();
    expect(messagesOf(issues)).toEqual([
      'Plugin not loaded: its manifest .claude-plugin/plugin.json was not read, because it could not be checked (ELOOP)',
    ]);
  });

  test('a manifest that resolves outside the plugin and its marketplace: nothing loads', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': { symlink: '/home/user/dotfiles/p/.claude-plugin/plugin.json' },
      'commands/hi.md': 'Hi.',
    });
    const roots = { pluginRoot: '/home/user/market/p', marketplaceRoot: '/home/user/market' };
    const { plugin, issues } = await loadPlugin({ files, fallbackName: 'p', roots });
    expect(plugin).toBeUndefined();
    expect(messagesOf(issues)).toEqual([
      'Plugin not loaded: its manifest /home/user/market/p/.claude-plugin/plugin.json was not read, because it resolves to /home/user/dotfiles/p/.claude-plugin/plugin.json, outside the plugin directory and its marketplace directory',
    ]);
  });
});
