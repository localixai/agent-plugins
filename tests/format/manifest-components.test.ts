/**
 * plugins/manifest-reference.md § Fields (component keys), § Component path forms, § Path rules.
 * Paths are relative to the plugin root and start with `./`; `skills` also takes `.`, and
 * `mcpServers` an `https://` bundle URL. A path with `..` is rejected with the validator's message.
 */
import { expect, test } from 'bun:test';
import { readManifest } from '../../src/format/index.ts';
import { spec } from '../support/spec.ts';

const loads = (raw: Record<string, unknown>) => readManifest({ name: 'demo', ...raw });
const TRAVERSAL = 'Path contains ".." which could be a path traversal attempt';

const PATH_FIELDS = ['skills', 'commands', 'agents', 'outputStyles', 'workflows'] as const;

for (const field of PATH_FIELDS) {
  spec(`manifest.field.${field}`, () => {
    const value = field === 'agents' ? './custom/reviewer.md' : './custom/';
    test('takes a path', () => {
      expect(loads({ [field]: value }).manifest?.[field]).toEqual(value);
    });
    test('takes an array of paths', () => {
      expect(loads({ [field]: [value] }).manifest?.[field]).toEqual([value]);
    });
    test('a path without the ./ prefix fails', () => {
      expect(loads({ [field]: 'custom/x.md' }).manifest).toBeUndefined();
    });
    test('a path with .. fails with the validator message', () => {
      const result = loads({ [field]: './../outside.md' });
      expect(result.manifest).toBeUndefined();
      expect(result.issues.map((found) => found.message)).toContain(
        `${TRAVERSAL}: ./../outside.md`,
      );
    });
  });
}

spec('manifest.field.skills', () => {
  test('also accepts "." and "./" for the plugin root', () => {
    expect(loads({ skills: ['.', './'] }).manifest?.skills).toEqual(['.', './']);
  });
});

spec('manifest.field.agents', () => {
  test('entries must be .md files', () => {
    expect(loads({ agents: './agents/' }).manifest).toBeUndefined();
  });
});

spec('manifest.field.commands', () => {
  test('takes an object map of command name to source or content', () => {
    const commands = {
      status: { source: './commands/status.md', argumentHint: '[env]' },
      about: { content: 'Explain what this plugin provides.' },
    };
    expect(loads({ commands }).manifest?.commands).toEqual(commands);
  });
});

for (const field of ['hooks', 'mcpServers', 'lspServers'] as const) {
  spec(`manifest.field.${field}`, () => {
    test('takes a .json path, an inline object, or an array mixing both', () => {
      // An inline LSP map is checked as one (spec §9.1.1 item 3), so it must be a valid one.
      const inline =
        field === 'lspServers'
          ? { go: { command: 'gopls', extensionToLanguage: { '.go': 'go' } } }
          : { anything: {} };
      for (const value of ['./config/x.json', inline, ['./config/x.json', inline]]) {
        expect(loads({ [field]: value }).manifest?.[field]).toEqual(value);
      }
    });
    // Spec §2.5 item 2, §17.18.12: a server file path with `..` is not read and the plugin loads.
    test(field === 'hooks' ? 'a path with .. fails' : 'a path with .. is not read', () => {
      const result = loads({ [field]: './../x.json' });
      if (field === 'hooks') expect(result.manifest).toBeUndefined();
      else expect(result.manifest).toEqual({ name: 'demo' });
    });
  });
}

spec('manifest.field.mcpServers', () => {
  test('also accepts an https:// bundle URL', () => {
    const url = 'https://example.com/server.mcpb';
    expect(loads({ mcpServers: url }).manifest?.mcpServers).toBe(url);
  });
});

spec(['manifest.field.experimental.themes', 'manifest.field.experimental.monitors'], () => {
  test('load under experimental without a warning', () => {
    const result = loads({ experimental: { themes: './themes/', monitors: './m.json' } });
    expect(result.issues).toEqual([]);
  });
  test('a top-level key still loads, with a warning', () => {
    const result = loads({ themes: './themes/', monitors: [] });
    expect(result.manifest).toMatchObject({ themes: './themes/', monitors: [] });
    expect(result.issues.map((found) => [found.severity, found.path])).toEqual([
      ['warning', 'themes'],
      ['warning', 'monitors'],
    ]);
  });
});

spec('manifest.field.experimental.evals', () => {
  test('is not a component path: it needs no ./ prefix', () => {
    expect(loads({ experimental: { evals: 'quality/evals' } }).issues).toEqual([]);
  });
});
