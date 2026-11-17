/** What the validator says about links inside a plugin (oracle 2.1.319): it reads none and warns where Claude Code would follow. */
import { describe, expect, test } from 'bun:test';
import { validatePlugin } from '../../../src/index.ts';
import { memoryFiles } from '../../support/ports.ts';

type Tree = Parameters<typeof memoryFiles>[0];

const MANIFEST = JSON.stringify({
  name: 'p',
  version: '1.0.0',
  description: 'd',
  author: { name: 'a' },
});
const plugin = async (tree: Tree) =>
  (
    await validatePlugin({
      files: memoryFiles({ '.claude-plugin/plugin.json': MANIFEST, ...tree }),
    })
  ).issues;

describe('folders and entries that are links', () => {
  test('a skills folder that is a link is not read and says so', async () => {
    const found = await plugin({ skills: { symlink: 'real' }, 'real/a/SKILL.md': 'text' });
    expect(found.map((issue) => [issue.severity, issue.file, issue.path])).toEqual([
      ['warning', 'skills', 'directory'],
    ]);
    expect(found[0]?.message).toStartWith(
      'This directory is a symlink, so the validator read nothing in it.',
    );
  });

  test('skill folders that are links are counted, one by one', async () => {
    const one = await plugin({ 'skills/a': { symlink: 'real' }, 'real/SKILL.md': 'text' });
    expect(one[0]?.message).toStartWith(
      '1 entry here is a symlink and was not read — components are read without following symlinks.',
    );
    const two = await plugin({
      'skills/a': { symlink: 'real' },
      'skills/b': { symlink: 'real' },
      'real/SKILL.md': 'text',
    });
    expect(two[0]?.message).toStartWith('2 entries here are symlinks and were not read');
  });

  test('a SKILL.md that is a link is a component that was not read', async () => {
    const found = await plugin({ 'skills/a/SKILL.md': { symlink: 'real.md' }, 'real.md': 'text' });
    expect(found[0]?.message).toStartWith(
      '1 component here was not read — the path is not a regular file',
    );
  });

  test('links among agents and commands are skipped without a word', async () => {
    const found = await plugin({
      agents: { symlink: 'real' },
      'commands/c.md': { symlink: 'x.md' },
      'x.md': 'text',
      'real/a.md': 'text',
    });
    expect(found.map((issue) => [issue.file, issue.path])).toEqual([['agents', 'directory']]);
  });

  test('a hooks folder or file that is a link is a warning on hooks.json', async () => {
    const folder = await plugin({ hooks: { symlink: 'real' }, 'real/hooks.json': '{"hooks":{}}' });
    expect(folder[0]?.message).toStartWith('The hooks directory is a link');
    const file = await plugin({
      'hooks/hooks.json': { symlink: 'real.json' },
      'real.json': '{"hooks":{}}',
    });
    expect(file[0]?.message).toStartWith('hooks.json is not a regular file');
    expect([file[0]?.file, file[0]?.path]).toEqual(['hooks/hooks.json', 'directory']);
  });

  test('an .mcp.json that is a link is a warning on the file', async () => {
    const found = await plugin({ '.mcp.json': { symlink: 'real.json' }, 'real.json': '{}' });
    expect(found[0]?.message).toStartWith('The file is not a regular file');
  });

  test('an .mcp.json past 1,048,576 bytes is not read and is a warning on the file (J8.27)', async () => {
    const big = JSON.stringify({ mcpServers: { a: 5 }, pad: 'x'.repeat(1_048_576) });
    const found = await plugin({ '.mcp.json': big });
    expect(found.map((issue) => [issue.severity, issue.file, issue.path, issue.message])).toEqual([
      [
        'warning',
        '.mcp.json',
        'directory',
        'The file is past the size cap and was not read — MCP server files are read without following symlinks and are capped at 1048576 bytes. The plugin loader has neither limit, so validate the real file separately.',
      ],
    ]);
    const atCap = await plugin({ '.mcp.json': JSON.stringify({ mcpServers: { a: 5 } }) });
    expect(atCap.map((issue) => issue.severity)).toEqual(['error']);
  });
});

describe('a project .claude folder', () => {
  test('uses its own wording, for a session loading it does follow links', async () => {
    const found = await validatePlugin({
      files: memoryFiles({ '.claude/skills/a': { symlink: 'real' }, 'real/SKILL.md': 'text' }),
    });
    expect(found.target).toBe('components');
    expect(found.issues[0]?.message).toContain('A session loading this directory does follow them');
  });
});
