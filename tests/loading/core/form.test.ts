/**
 * Telling the forms apart (Agent Plugins Extended §1.2, §1.3 and its table, §1.3.1; J1.1, J1.2), and
 * the loader of an Extended client that follows the decision: a Claude Code plugin loads exactly as
 * `loadPlugin` loads it, a core package by core §4–§10, a dual package as Claude Code only. The
 * chapter's requirement ids are Claude Code's layout ids, which the form decision does not prove, so
 * these are judgment-style tests outside `spec(...)` (docs/SPEC.md, "The specification").
 */
import { describe, expect, test } from 'bun:test';
import { checkDualPackage, detectPluginForm, loadPluginPackage } from '../../../src/entry/core.ts';
import { loadPlugin } from '../../../src/index.ts';
import { json } from '../../support/plugin-folder.ts';
import { loaded as entryOf, rawEntry } from '../../support/marketplace-entries.ts';
import { memoryFiles } from '../../support/ports.ts';

const SCHEMA = 'https://agent-plugins.org/schemas/1.1.0/plugin.schema.json';
const CORE = json({ $schema: SCHEMA, name: 'core-tools' });
const CLAUDE = json({ name: 'cc-tools' });
const SKILL = '---\nname: deploy\ndescription: Deploys.\n---\nDeploy.\n';

const form = (tree: Record<string, string | { symlink: string }>) =>
  detectPluginForm({ files: memoryFiles(tree) });

describe('§1.3: the form table', () => {
  test('.claude-plugin/plugin.json alone: a Claude Code plugin', async () => {
    expect(await form({ '.claude-plugin/plugin.json': CLAUDE })).toEqual({
      form: 'claude-code',
      dual: false,
    });
  });

  test('both manifests: a Claude Code plugin that is a dual package', async () => {
    expect(await form({ '.claude-plugin/plugin.json': CLAUDE, 'plugin.json': CORE })).toEqual({
      form: 'claude-code',
      dual: true,
    });
  });

  test('a root plugin.json with a core $schema alone: a core package', async () => {
    expect(await form({ 'plugin.json': CORE })).toEqual({ form: 'core', dual: false });
  });

  test('a canonical identifier of an unsupported version still makes a core package', async () => {
    const old = json({
      $schema: 'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json',
      name: 'x',
    });
    expect(await form({ 'plugin.json': old })).toEqual({ form: 'core', dual: false });
  });

  test('J1.1: a root plugin.json without a core $schema is an ordinary file', async () => {
    for (const text of [
      json({ name: 'x' }),
      json({ $schema: 'https://example.com/s.json' }),
      'not json',
      '[]',
    ]) {
      expect(await form({ 'plugin.json': text, 'skills/deploy/SKILL.md': SKILL })).toEqual({
        form: 'claude-code',
        dual: false,
      });
    }
  });

  test('neither manifest: a Claude Code plugin without a manifest', async () => {
    expect(await form({ 'skills/deploy/SKILL.md': SKILL })).toEqual({
      form: 'claude-code',
      dual: false,
    });
  });

  test('judgment: a root plugin.json linked outside the root is not read', async () => {
    expect(await form({ 'plugin.json': { symlink: '../elsewhere/plugin.json' } })).toEqual({
      form: 'claude-code',
      dual: false,
    });
  });
});

describe('§1.2: loading in the decided form', () => {
  test('a Claude Code plugin loads exactly as loadPlugin loads it', async () => {
    const tree = { '.claude-plugin/plugin.json': CLAUDE, 'skills/deploy/SKILL.md': SKILL };
    const loaded = await loadPluginPackage({ files: memoryFiles(tree) });
    expect(loaded).toEqual({
      form: 'claude-code',
      dual: false,
      loaded: await loadPlugin({ files: memoryFiles(tree) }),
    });
  });

  test('a core package loads by the core', async () => {
    const loaded = await loadPluginPackage({
      files: memoryFiles({ 'plugin.json': CORE, 'skills/deploy/SKILL.md': SKILL }),
    });
    expect(loaded.form).toBe('core');
    expect(
      loaded.form === 'core' && loaded.loaded.package?.skills.map((one) => one.qualifiedName),
    ).toEqual(['core-tools:deploy']);
  });

  test("J18.15: a marketplace entry's component fields and strict: false are reported for a core package, which loads as the core says", async () => {
    const entry = entryOf(
      rawEntry({
        name: 'core-tools',
        strict: false,
        description: 'From the entry',
        commands: ['./extra'],
        mcpServers: { x: { command: 'x' } },
      }),
    );
    const loaded = await loadPluginPackage({
      files: memoryFiles({ 'plugin.json': CORE, 'skills/deploy/SKILL.md': SKILL }),
      entry,
    });
    if (loaded.form !== 'core') throw new Error('not loaded as a core package');
    expect(loaded.loaded.package?.skills.map((one) => one.qualifiedName)).toEqual([
      'core-tools:deploy',
    ]);
    expect(loaded.loaded.issues.map(({ severity, path }) => ({ severity, path }))).toEqual([
      { severity: 'error', path: 'commands' },
      { severity: 'error', path: 'mcpServers' },
      { severity: 'error', path: 'strict' },
    ]);
  });

  test('J1.2: a dual package loads as Claude Code only; mcp.json and hooks.json are not read', async () => {
    const files = memoryFiles({
      '.claude-plugin/plugin.json': CLAUDE,
      'plugin.json': CORE,
      'mcp.json': json({ $schema: 'x', mcpServers: {} }),
      'hooks.json': json({ $schema: 'x', hooks: {} }),
    });
    const loaded = await loadPluginPackage({ files });
    expect(loaded.form).toBe('claude-code');
    expect(loaded.dual).toBe(true);
    expect(loaded.form === 'claude-code' && loaded.loaded.plugin?.name).toBe('cc-tools');
    expect(files.reads.filter((path) => path === 'mcp.json' || path === 'hooks.json')).toEqual([]);
  });
});

describe('§18.3.1: the dual-package check of a folder', () => {
  test('a dual package is checked; any other root gives nothing', async () => {
    const dual = await checkDualPackage({
      files: memoryFiles({ '.claude-plugin/plugin.json': CLAUDE, 'plugin.json': CORE }),
    });
    expect(dual.dual).toBe(true);
    expect(dual.issues.map((one) => one.path)).toEqual(['description', 'name']);
    expect(await checkDualPackage({ files: memoryFiles({ 'plugin.json': CORE }) })).toEqual({
      dual: false,
      issues: [],
    });
  });
});
