/**
 * Spec §2.2 and §2.4 (`engines`, `network`; plugins/manifest-reference.md, "Fields"; changelog
 * 2.1.323) and Judgment J2.10: `engines["claude-code"]` is the node-semver range of Claude Code
 * versions the plugin works on; outside it the plugin does not load, except from a local folder,
 * which loads with a note. A value that is not a range is ignored with a validator error and a note.
 * `network.hosts` lists the hosts the plugin reaches; it neither allows nor blocks one.
 */
import { describe, expect, test } from 'bun:test';
import { loadPlugin } from '../../src/index.ts';
import { readManifest, validateManifest } from '../../src/format/index.ts';
import { engineInstallSuffix, newestFittingVersion } from '../../src/format/engines.ts';
import { declaredHosts } from '../../src/format/network.ts';
import { json } from '../support/plugin-folder.ts';
import { memoryFiles } from '../support/ports.ts';
import { spec } from '../support/spec.ts';

const loadWith = (manifest: Record<string, unknown>, facts: object = {}) =>
  loadPlugin({
    files: memoryFiles({ '.claude-plugin/plugin.json': json({ name: 'p', ...manifest }) }),
    ...facts,
  });

const findings = (raw: Record<string, unknown>) =>
  validateManifest({
    name: 'p',
    version: '1.0.0',
    description: 'd',
    author: { name: 'a' },
    ...raw,
  });

spec('manifest.field.engines', () => {
  test('a range that fits the client loads', async () => {
    const loaded = await loadWith(
      { engines: { 'claude-code': '>=2.1.0' } },
      { clientVersion: '2.1.330' },
    );
    expect(loaded.plugin?.manifest?.engines).toEqual({ 'claude-code': '>=2.1.0' });
    expect(loaded.notes).toBeUndefined();
  });

  test('a range that excludes the client keeps the plugin from loading', async () => {
    const loaded = await loadWith(
      { engines: { 'claude-code': '>=2.1.400' } },
      { clientVersion: '2.1.330' },
    );
    expect(loaded.plugin).toBeUndefined();
    expect(loaded.issues).toContainEqual(
      expect.objectContaining({ severity: 'error', path: 'engines.claude-code' }),
    );
  });

  test('read from a local folder, it loads anyway with the engine-range-exempt note', async () => {
    const loaded = await loadWith(
      { engines: { 'claude-code': '>=9.0.0' } },
      { clientVersion: '2.1.330', localFolder: true },
    );
    expect(loaded.plugin?.name).toBe('p');
    expect(loaded.notes).toEqual([
      {
        type: 'engine-range-exempt',
        message:
          "It needs Claude Code >=9.0.0, this is 2.1.330. It loads anyway, because it's read from a local folder",
      },
    ]);
  });

  test("the install line's note for such a plugin", () => {
    expect(engineInstallSuffix('p', '>=9.0.0', '2.1.330')).toBe(
      `. Note: "p" needs Claude Code >=9.0.0, this is 2.1.330, but loads anyway because it's read from a local folder`,
    );
  });

  test('a value that is not a range is ignored: the plugin loads with the engine-range-invalid note', async () => {
    const loaded = await loadWith(
      { engines: { 'claude-code': 'banana' } },
      { clientVersion: '2.1.330' },
    );
    expect(loaded.plugin?.name).toBe('p');
    expect(loaded.notes).toEqual([
      {
        type: 'engine-range-invalid',
        message: `Its Claude Code range "banana" isn't a version range, so it's ignored`,
      },
    ]);
  });

  test('the validator reports a value that is not a range as an error at engines.claude-code', () => {
    expect(findings({ engines: { 'claude-code': 'banana' } })).toContainEqual({
      severity: 'error',
      path: 'engines.claude-code',
      message:
        'Must be a version range such as ">=2.1.0"; got "banana". It will be ignored at load time.',
    });
    expect(findings({ engines: { 'claude-code': 5 } })).toContainEqual({
      severity: 'error',
      path: 'engines.claude-code',
      message:
        'Must be a version range such as ">=2.1.0"; got number. It will be ignored at load time.',
    });
  });

  test('an engines that is not an object is a warning, and loading ignores it', async () => {
    expect(findings({ engines: 'x' })).toContainEqual({
      severity: 'warning',
      path: 'engines',
      message:
        '\'engines\' must be an object such as { "claude-code": ">=2.1.0" }; got string. It will be ignored at load time.',
    });
    const loaded = await loadWith({ engines: 'x' }, { clientVersion: '2.1.330' });
    expect(loaded.plugin?.manifest).toEqual({ name: 'p' });
  });

  test('a valid range is no finding, and engines is a known field', () => {
    expect(findings({ engines: { 'claude-code': '>=2.1.0' } })).toEqual([]);
  });
});

spec('manifest.field.network', () => {
  test('the hosts load as declared; the plugin loads', async () => {
    const loaded = await loadWith({ network: { hosts: ['api.example.com', '*.example.org'] } });
    expect(declaredHosts(loaded.plugin?.manifest?.network)).toEqual([
      'api.example.com',
      '*.example.org',
    ]);
    expect(findings({ network: { hosts: ['api.example.com'] } })).toEqual([]);
  });

  test('a value that is not an object with a list of hosts is a warning, ignored at load', async () => {
    const message =
      '\'network\' must be an object with a list of hosts, such as { "hosts": ["api.example.com"] }. It will be ignored at load time.';
    for (const network of [5, { hosts: 'x' }]) {
      expect(findings({ network })).toContainEqual({
        severity: 'warning',
        path: 'network',
        message,
      });
    }
    expect((await loadWith({ network: 5 })).plugin?.manifest).toEqual({ name: 'p' });
  });

  test('a host with a scheme or path, or not plain ASCII, is a warning at its index; an unknown key is one too', () => {
    const found = findings({
      network: { hosts: ['http://x', 'ok.example.com', 'a/b', 'café.fr'], x: 1 },
    });
    expect(found.map((one) => one.path)).toEqual([
      'network.hosts[0]',
      'network.hosts[2]',
      'network.hosts[3]',
      'network.x',
    ]);
    expect(found.every((one) => one.severity === 'warning')).toBe(true);
    expect(
      found[0]?.message.startsWith('Must be a host as sandbox.network.allowedDomains takes it'),
    ).toBe(true);
    expect(found[3]?.message).toBe("Unknown field 'x'. Claude Code ignores it at load time.");
    expect(declaredHosts({ hosts: ['http://x', 'ok.example.com'] })).toEqual(['ok.example.com']);
  });
});

describe('judgment J2.10: engines keys other than claude-code', () => {
  test('are ignored, by the validator and at load', async () => {
    expect(findings({ engines: { node: '>=1' } })).toEqual([]);
    const loaded = await loadWith({ engines: { node: '>=99' } }, { clientVersion: '2.1.330' });
    expect(loaded.plugin?.name).toBe('p');
  });

  test('without the client version, only the invalid-range note can be known', async () => {
    const loaded = await loadWith({ engines: { 'claude-code': '>=9.0.0' } });
    expect(loaded.plugin?.name).toBe('p');
    expect(loaded.notes).toBeUndefined();
  });

  test('readManifest keeps engines as written', () => {
    expect(readManifest({ name: 'p', engines: { 'claude-code': '>=2.1.0' } }).manifest).toEqual({
      name: 'p',
      engines: { 'claude-code': '>=2.1.0' },
    });
  });
});

spec('manifest.field.engines', () => {
  test('an install or update picks the newest version whose range fits', () => {
    const candidates = [
      { version: '1.0.0', engines: { 'claude-code': '>=2.1.0' } },
      { version: '1.2.0', engines: { 'claude-code': '>=2.1.400' } },
      { version: '1.1.0' },
      { version: '0.9.0', engines: { 'claude-code': 'banana' } },
    ];
    expect(newestFittingVersion(candidates, { clientVersion: '2.1.330', localFolder: false })).toBe(
      '1.1.0',
    );
    expect(
      newestFittingVersion(candidates.slice(1, 2), {
        clientVersion: '2.1.330',
        localFolder: false,
      }),
    ).toBeUndefined();
    expect(newestFittingVersion(candidates, { clientVersion: '2.1.330', localFolder: true })).toBe(
      '1.2.0',
    );
  });
});
