/**
 * Loading a core package (Agent Plugins 1.1.0, `spec/1.1.0.md`): core §4.1 (containment and the
 * narrowest failure boundary), §4.3.1 (icons as data), §5.1 (the manifest before any component),
 * §6.1–§6.2 (fixed locations, missing and wrong-kind locations), §7.1 (skills: one level of
 * `skills/`, an invalid skill skipped), §7.2.2 and §7.3.2 (command containment and resolution), §8
 * (extension data and directories ignored unless implemented), §10.1 (component versions match),
 * §11.3 (independent failures). Judgment-style tests outside `spec(...)` (docs/SPEC.md).
 */
import { describe, expect, test } from 'bun:test';
import { loadCorePackage } from '../../../src/entry/core.ts';
import type { MemoryEntry } from '../../../src/testing/ports/files.ts';
import { json } from '../../support/plugin-folder.ts';
import { memoryFiles } from '../../support/ports.ts';

const schema = (document: string) =>
  `https://agent-plugins.org/schemas/1.1.0/${document}.schema.json`;
const MANIFEST = json({ $schema: schema('plugin'), name: 'reports-plugin' });
const SKILL = (name: string) => `---\nname: ${name}\ndescription: Does ${name}.\n---\nBody\n`;
const mcpFile = (servers: Record<string, unknown>) =>
  json({ $schema: schema('mcp'), mcpServers: servers });
const hooksFile = (hooks: Record<string, unknown>) => json({ $schema: schema('hooks'), hooks });

const load = (
  tree: Record<string, MemoryEntry>,
  options: { extensions?: string[]; iconFormats?: string[] } = {},
) => loadCorePackage({ files: memoryFiles({ 'plugin.json': MANIFEST, ...tree }), ...options });

describe('core §5.1, §4.1 boundary 1: the manifest first', () => {
  test('the example of core §6.1 loads its three component types', async () => {
    const { package: loaded, issues } = await load({
      'skills/summarize/SKILL.md': SKILL('summarize'),
      'mcp.json': mcpFile({ s: { type: 'stdio', command: 'node' } }),
      'hooks.json': hooksFile({ Stop: [{ command: 'node' }] }),
    });
    expect(issues).toEqual([]);
    expect(loaded?.name).toBe('reports-plugin');
    expect(loaded?.version).toBe('1.1.0');
    expect(loaded?.skills.map((one) => [one.name, one.qualifiedName, one.path])).toEqual([
      ['summarize', 'reports-plugin:summarize', 'skills/summarize/SKILL.md'],
    ]);
    expect(Object.keys(loaded?.mcpServers ?? {})).toEqual(['s']);
    expect(loaded?.hooks.Stop).toEqual([{ command: 'node' }]);
  });

  test('a package without plugin.json, or with one that is not JSON, is rejected', async () => {
    expect(
      (await loadCorePackage({ files: memoryFiles({ 'skills/a/SKILL.md': SKILL('a') }) })).package,
    ).toBeUndefined();
    const broken = await loadCorePackage({ files: memoryFiles({ 'plugin.json': '{' }) });
    expect(broken.package).toBeUndefined();
    expect(broken.issues[0]?.path).toBe('plugin.json');
  });

  test('a plugin.json that resolves outside the root rejects the package', async () => {
    const outside = await loadCorePackage({
      files: memoryFiles({ 'plugin.json': { symlink: '/elsewhere/plugin.json' } }),
    });
    expect(outside.package).toBeUndefined();
  });

  test('an invalid manifest rejects the package before any component is read', async () => {
    const files = memoryFiles({
      'plugin.json': json({ $schema: schema('plugin'), name: 'Bad Name' }),
      'skills/a/SKILL.md': SKILL('a'),
      'mcp.json': mcpFile({}),
    });
    const result = await loadCorePackage({ files });
    expect(result.package).toBeUndefined();
    expect(files.reads).toEqual(['plugin.json']);
  });

  test('core §6.1: no Claude Code location is read', async () => {
    const files = memoryFiles({
      'plugin.json': MANIFEST,
      '.mcp.json': mcpFile({}),
      'hooks/hooks.json': hooksFile({}),
      'commands/x.md': 'x',
    });
    await loadCorePackage({ files });
    expect(files.reads).toEqual(['plugin.json']);
  });
});

describe('core §6.2: missing and wrong-kind locations', () => {
  test('absent locations are no error', async () => {
    const result = await load({});
    expect(result.issues).toEqual([]);
    expect(result.package?.skills).toEqual([]);
  });

  test('skills that is a file, or mcp.json and hooks.json that are directories, invalidate that type only', async () => {
    const result = await load({
      skills: 'not a directory',
      'mcp.json/x': '{}',
      'hooks.json': hooksFile({ Stop: [{ command: 'node' }] }),
    });
    expect(result.package?.hooks.Stop).toHaveLength(1);
    expect(result.issues.map((one) => one.path)).toEqual(['skills', 'mcp.json']);
  });

  test('core §4.1 boundary 2: a fixed location linked outside the root invalidates that type', async () => {
    const result = await load({
      'mcp.json': { symlink: '/etc/mcp.json' },
      'skills/a/SKILL.md': SKILL('a'),
    });
    expect(result.package?.skills).toHaveLength(1);
    expect(result.package?.mcpServers).toEqual({});
    expect(result.issues.map((one) => one.path)).toEqual(['mcp.json']);
  });
});

describe('core §7.1: skills', () => {
  test('only immediate children of skills/ with a SKILL.md are skills; nothing deeper', async () => {
    const result = await load({
      'skills/a/SKILL.md': SKILL('a'),
      'skills/group/nested/SKILL.md': SKILL('nested'),
      'skills/SKILL.md': SKILL('skills'),
      'skills/b/skill.md': SKILL('b'),
    });
    expect(result.package?.skills.map((one) => one.name)).toEqual(['a']);
  });

  test('a skill that fails Agent Skills is skipped and reported; the others load', async () => {
    const result = await load({ 'skills/a/SKILL.md': SKILL('a'), 'skills/b/SKILL.md': SKILL('c') });
    expect(result.package?.skills.map((one) => one.name)).toEqual(['a']);
    expect(result.issues).toEqual([
      {
        severity: 'error',
        path: 'skills/b/SKILL.md',
        message: 'Skill "b" is skipped: "name" "c" must match its directory "b"',
      },
    ]);
  });

  test('core §4.1 boundary 3: a SKILL.md that resolves outside the root is skipped', async () => {
    const result = await load({
      'skills/a/SKILL.md': { symlink: '/outside/SKILL.md' },
      'skills/b/SKILL.md': SKILL('b'),
    });
    expect(result.package?.skills.map((one) => one.name)).toEqual(['b']);
    expect(result.issues.map((one) => one.path)).toEqual(['skills/a/SKILL.md']);
  });
});

describe('core §7.2.2, §7.3.2 and §4.1 boundaries 4–5: commands and cwd on disk', () => {
  test('a ./ command must be a file inside the root; a bare name resolves at launch', async () => {
    const result = await load({
      'bin/server': '#!/bin/sh',
      'bin/outside': { symlink: '/usr/bin/env' },
      'mcp.json': mcpFile({
        ok: { type: 'stdio', command: './bin/server' },
        bare: { type: 'stdio', command: 'node' },
        missing: { type: 'stdio', command: './bin/none' },
        escapes: { type: 'stdio', command: './bin/outside' },
      }),
    });
    expect(Object.keys(result.package?.mcpServers ?? {})).toEqual(['ok', 'bare']);
    expect(result.issues.map((one) => one.path)).toEqual([
      'mcpServers.missing',
      'mcpServers.escapes',
    ]);
  });

  test('a cwd under the root that leads outside through a link invalidates the server', async () => {
    const result = await load({
      data: { symlink: '/tmp' },
      'mcp.json': mcpFile({
        linked: { type: 'stdio', command: 'node', cwd: '${PLUGIN_ROOT}/data/x' },
        fine: { type: 'stdio', command: 'node', cwd: './not-yet-there' },
      }),
    });
    expect(Object.keys(result.package?.mcpServers ?? {})).toEqual(['fine']);
  });

  test('a handler whose ./ command is missing or outside is skipped; its siblings load', async () => {
    const result = await load({
      'hooks/check': '#!/bin/sh',
      'hooks.json': hooksFile({
        PreToolUse: [{ command: './hooks/check' }, { command: './hooks/gone' }],
      }),
    });
    expect(result.package?.hooks.PreToolUse).toEqual([{ command: './hooks/check' }]);
    expect(result.issues.map((one) => one.path)).toEqual(['hooks.PreToolUse[1]']);
  });

  test('after a handler skipped by its shape, a later one is named by its own index (core §7.3.2)', async () => {
    const result = await load({
      'hooks/check': '#!/bin/sh',
      'hooks.json': hooksFile({
        PreToolUse: [{ command: 'a b' }, { command: './hooks/check' }, { command: './hooks/gone' }],
      }),
    });
    expect(result.package?.hooks.PreToolUse).toEqual([{ command: './hooks/check' }]);
    expect(result.issues.map((one) => one.path)).toEqual([
      'hooks.PreToolUse[0]',
      'hooks.PreToolUse[2]',
    ]);
  });
});

describe('core §10.1: component versions match plugin.json', () => {
  test('a mismatched mcp.json disables MCP and leaves hooks loading', async () => {
    const result = await load({
      'mcp.json': json({
        $schema: 'https://agent-plugins.org/schemas/1.2.0/mcp.schema.json',
        mcpServers: {},
      }),
      'hooks.json': hooksFile({ Stop: [{ command: 'node' }] }),
    });
    expect(result.package?.hooks.Stop).toHaveLength(1);
    expect(result.issues.map((one) => one.path)).toEqual(['mcp.json']);
  });
});

describe('core §4.3.1, §8: icons and extension directories are data', () => {
  test('icon candidates are found; a broken or outside one never affects the package', async () => {
    const result = await load(
      {
        'assets/icon.png': 'png',
        'assets/icon.svg': { symlink: '/x.svg' },
        'assets/Icon.png': 'x',
      },
      { iconFormats: ['svg'] },
    );
    expect(result.package?.icons).toEqual(['assets/icon.png']);
    expect(result.issues).toEqual([]);
  });

  test('only the directories of implemented namespaces are found, never assets/', async () => {
    const tree = {
      'com.example.client/hooks/hooks.json': '{}',
      'org.other/x': 'y',
      'assets/icon.png': 'p',
    };
    expect((await load(tree)).package?.extensionDirectories).toEqual([]);
    const found = await load(tree, { extensions: ['com.example.client', 'assets'] });
    expect(found.package?.extensionDirectories).toEqual(['com.example.client']);
  });

  test('manifest extensions of unimplemented namespaces are dropped', async () => {
    const files = memoryFiles({
      'plugin.json': json({
        $schema: schema('plugin'),
        name: 'x',
        extensions: { 'a.b': { k: 1 }, 'c.d': { k: 2 } },
      }),
    });
    const result = await loadCorePackage({ files, extensions: ['a.b'] });
    expect(result.package?.manifest.extensions).toEqual({ 'a.b': { k: 1 } });
  });
});
