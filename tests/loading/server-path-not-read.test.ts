/**
 * Spec §17.18.12 (errors.md, "A server file path not read") and §2.5 item 2: an `mcpServers` or
 * `lspServers` file path that is absolute or has a `..` part is not read, even when it stays inside
 * the plugin; the rest of the plugin loads without the servers that file defines. The validator
 * reports the same path as `Path contains ".." which could be a path traversal attempt: <path>`.
 */
import { expect, test } from 'bun:test';
import { loadPlugin } from '../../src/index.ts';
import { validateManifest } from '../../src/format/index.ts';
import { json } from '../support/plugin-folder.ts';
import { memoryFiles } from '../support/ports.ts';
import { spec } from '../support/spec.ts';

const SERVERS = json({ mcpServers: { api: { command: 'node' } } });

const load = (mcpServers: unknown, extra: Record<string, string> = {}) =>
  loadPlugin({
    files: memoryFiles({
      '.claude-plugin/plugin.json': json({ name: 'p', mcpServers }),
      'x.json': SERVERS,
      'servers/x.json': SERVERS,
      'skills/s/SKILL.md': '---\ndescription: d\n---\nbody',
      ...extra,
    }),
  });

spec('cli.errors.Server file path not read', () => {
  test('a path with a .. part that stays inside is not read; the plugin loads without its servers', async () => {
    const loaded = await load('./sub/../x.json');
    expect(loaded.plugin?.mcpServers).toEqual({});
    expect(loaded.plugin?.skills.map((skill) => skill.name)).toEqual(['s']);
    expect(loaded.issues).toContainEqual(
      expect.objectContaining({
        message: 'mcp-servers path not read: ./sub/../x.json is absolute or has a ".." part',
      }),
    );
    expect(loaded.issues.some((issue) => issue.severity === 'error')).toBe(false);
  });

  test('an absolute path is not read either', async () => {
    const loaded = await load('/home/user/x.json');
    expect(loaded.plugin?.mcpServers).toEqual({});
    expect(loaded.issues).toContainEqual(
      expect.objectContaining({
        message: 'mcp-servers path not read: /home/user/x.json is absolute or has a ".." part',
      }),
    );
  });

  test('in a list, only that path is left out', async () => {
    const loaded = await load(['./sub/../x.json', './servers/x.json']);
    expect(Object.keys(loaded.plugin?.mcpServers ?? {})).toEqual(['api']);
  });

  test('the fix, a ./ path without .., loads the servers', async () => {
    expect(Object.keys((await load('./servers/x.json')).plugin?.mcpServers ?? {})).toEqual(['api']);
  });

  test('lspServers paths follow the same rule', async () => {
    const loaded = await loadPlugin({
      files: memoryFiles({
        '.claude-plugin/plugin.json': json({ name: 'p', lspServers: './a/../l.json' }),
      }),
    });
    expect(loaded.plugin?.lspServers).toEqual({});
    expect(loaded.issues).toContainEqual(
      expect.objectContaining({
        message: 'lsp-servers path not read: ./a/../l.json is absolute or has a ".." part',
      }),
    );
  });

  test('the validator reports the path as a traversal error', () => {
    expect(validateManifest({ name: 'p', mcpServers: './sub/../x.json' })).toContainEqual(
      expect.objectContaining({
        severity: 'error',
        message: 'Path contains ".." which could be a path traversal attempt: ./sub/../x.json',
      }),
    );
  });
});
