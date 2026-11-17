/**
 * plugins/manifest-reference.md § Unrecognized fields ("An unknown key inside one is an error, and
 * the plugin doesn't load": `lspServers` configurations and `monitors` entries), § lspServers and
 * § monitors. The validator's wording and paths are the oracle's (claude plugin validate 2.1.325,
 * spec §9.1.3 and §9.2.2, spec/judgments.md `oracle 2.1.325`), tested outside spec(); that an
 * inline strict object with an unknown key keeps the plugin from loading is the docs', under spec().
 */
import { describe, expect, test } from 'bun:test';
import { checkPluginManifest } from '../../../src/format/validate/plugin-manifest.ts';
import { readManifest } from '../../../src/format/manifest.ts';
import { loadPlugin } from '../../../src/index.ts';
import { memoryFiles } from '../../support/ports.ts';
import { json } from '../../support/plugin-folder.ts';
import { spec } from '../../support/spec.ts';

const complete = { version: '1.0.0', description: 'Demo', author: { name: 'Team' } };
const GO = { command: 'gopls', extensionToLanguage: { '.go': 'go' } };
const MONITOR = { name: 'm', command: 'poll', description: 'd' };

const findings = (rest: Record<string, unknown>): [string, string][] =>
  checkPluginManifest({ name: 'p', ...complete, ...rest })
    .findings.filter((found) => found.severity === 'error')
    .map((found) => [found.path, found.message]);
const lsp = (change: Record<string, unknown>) =>
  findings({ lspServers: { go: { ...GO, ...change } } });
const monitors = (list: unknown) => findings({ experimental: { monitors: list } });

spec('manifest.rule.Unrecognized fields', () => {
  test('an unknown key in an inline lspServers configuration keeps the plugin from loading', async () => {
    const manifest = { name: 'p', lspServers: { go: { ...GO, foo: 1 } } };
    expect(readManifest(manifest).manifest).toBeUndefined();
    const files = memoryFiles({ '.claude-plugin/plugin.json': json(manifest) });
    expect((await loadPlugin({ files })).plugin).toBeUndefined();
  });

  test('an unknown key in an inline monitors entry keeps the plugin from loading, at either key', () => {
    for (const manifest of [
      { name: 'p', experimental: { monitors: [{ ...MONITOR, foo: 1 }] } },
      { name: 'p', monitors: [{ ...MONITOR, foo: 1 }] },
    ]) {
      expect(readManifest(manifest).manifest).toBeUndefined();
    }
  });

  test('a valid inline map and array load', () => {
    const manifest = readManifest({
      name: 'p',
      lspServers: { go: GO },
      experimental: { monitors: [MONITOR] },
    }).manifest;
    expect(manifest?.lspServers).toEqual({ go: GO });
    expect(manifest?.experimental?.monitors).toEqual([MONITOR]);
  });
});

describe('the inline lspServers table of the validator (oracle 2.1.325, spec §9.1.3)', () => {
  test('problems that keep the entry readable are reported at their own path', () => {
    expect(lsp({ foo: 1 })).toEqual([['lspServers.go', 'Unrecognized key: "foo"']]);
    expect(lsp({ foo: 1, bar: 2 })).toEqual([['lspServers.go', 'Unrecognized keys: "foo", "bar"']]);
    expect(lsp({ command: 'gopls serve' })).toEqual([
      ['lspServers.go.command', 'Command should not contain spaces. Use args array for arguments.'],
    ]);
    expect(lsp({ command: '' })).toEqual([
      ['lspServers.go.command', 'Too small: expected string to have >=1 characters'],
    ]);
    expect(lsp({ extensionToLanguage: {} })).toEqual([
      ['lspServers.go.extensionToLanguage', 'extensionToLanguage must have at least one mapping'],
    ]);
    expect(lsp({ startupTimeout: 0 })).toEqual([
      ['lspServers.go.startupTimeout', 'Too small: expected number to be >0'],
    ]);
    expect(lsp({ maxRestarts: -1 })).toEqual([
      ['lspServers.go.maxRestarts', 'Too small: expected number to be >=0'],
    ]);
  });

  test('a type error, a fraction or a key without its dot is a bare Invalid input at the key', () => {
    for (const change of [
      { extensionToLanguage: { go: 'go' } },
      { transport: 'tcp' },
      { requestTimeout: 1.5 },
      { env: { A: 1 } },
      { command: undefined },
    ]) {
      expect(lsp(change)).toEqual([['lspServers', 'Invalid input']]);
    }
    const { extensionToLanguage } = GO;
    expect(findings({ lspServers: { go: { extensionToLanguage } } })).toEqual([
      ['lspServers', 'Invalid input'],
    ]);
  });

  test('what passes, and the array form keeps the index', () => {
    for (const change of [
      { command: '/opt/my dir/gopls' },
      { transport: 'socket' },
      { maxRestarts: 0 },
      { initializationOptions: 5, settings: 'x' },
    ]) {
      expect(lsp(change)).toEqual([]);
    }
    expect(findings({ lspServers: [{ go: { ...GO, foo: 1 } }] })).toEqual([
      ['lspServers.0.go', 'Unrecognized key: "foo"'],
    ]);
  });
});

describe('the inline monitors checks of the validator (oracle 2.1.325, spec §9.2.2)', () => {
  test('an unknown key, an empty name, and the two `when` messages', () => {
    expect(monitors([{ ...MONITOR, foo: 1 }])).toEqual([
      ['experimental.monitors.0', 'Unrecognized key: "foo"'],
    ]);
    expect(monitors([{ ...MONITOR, name: '' }])).toEqual([
      ['experimental.monitors.0.name', 'Too small: expected string to have >=1 characters'],
    ]);
    expect(monitors([{ ...MONITOR, when: 'sometimes' }])).toEqual([
      ['experimental.monitors.0.when', 'Invalid string: must start with "on-skill-invoke:"'],
      ['experimental.monitors.0.when', 'on-skill-invoke: must specify a skill name'],
    ]);
    expect(monitors([{ ...MONITOR, when: 'on-skill-invoke:' }])).toEqual([
      ['experimental.monitors.0.when', 'on-skill-invoke: must specify a skill name'],
    ]);
    expect(monitors([{ ...MONITOR, when: 'on-skill-invoke:deploy' }])).toEqual([]);
  });

  test('a repeated name fails at the array; a missing field or an object is Invalid input', () => {
    expect(monitors([MONITOR, MONITOR])).toEqual([
      ['experimental.monitors', 'Monitor names must be unique within a plugin'],
    ]);
    expect(monitors([{ name: 'm', command: 'poll' }])).toEqual([
      ['experimental.monitors', 'Invalid input'],
    ]);
    expect(monitors({ a: 1 })).toEqual([['experimental.monitors', 'Invalid input']]);
  });

  test('the top-level key is checked the same way', () => {
    expect(findings({ monitors: [{ ...MONITOR, foo: 1 }] })).toEqual([
      ['monitors.0', 'Unrecognized key: "foo"'],
    ]);
  });

  test('a repeated inline name keeps the plugin from loading', () => {
    expect(
      readManifest({ name: 'p', experimental: { monitors: [MONITOR, MONITOR] } }).manifest,
    ).toBeUndefined();
  });
});
