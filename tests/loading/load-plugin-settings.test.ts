/**
 * plugins/components.md § Default settings ("`settings.json` applies and the manifest's `settings` is
 * ignored"); plugins/manifest-reference.md § `settings`, § User configuration (the options a manifest
 * declares), § Containment and existence (a path that is missing skips that component).
 *
 * Each test builds a plugin folder in memory and calls the real `loadPlugin`.
 */
import { describe, expect, test } from 'bun:test';
import { loadPlugin } from '../../src/index.ts';
import { memoryFiles } from '../support/ports.ts';
import { json, messagesOf, namesOf } from '../support/plugin-folder.ts';
import { spec } from '../support/spec.ts';

const load = async (tree: Record<string, string>, fallbackName?: string) =>
  loadPlugin({ files: memoryFiles(tree), ...(fallbackName && { fallbackName }) });

const manifest = (settings: unknown) => json({ name: 'p', settings });

describe('the manifest settings key (spec §2.4, §12.6; Judgments J2.9, J12.10)', () => {
  test('settings.json applies when it sets a supported key, and the manifest settings is ignored', async () => {
    const { plugin } = await load({
      '.claude-plugin/plugin.json': manifest({ agent: 'from-manifest' }),
      'settings.json': json({ agent: 'from-file' }),
    });
    expect(plugin?.settings).toEqual({ agent: 'from-file' });
  });

  test('a settings.json with no supported key leaves the manifest settings in force', async () => {
    const { plugin } = await load({
      '.claude-plugin/plugin.json': manifest({ agent: 'from-manifest' }),
      'settings.json': json({ theme: 'dark' }),
    });
    expect(plugin?.settings).toEqual({ agent: 'from-manifest' });
  });

  test('the inline settings key alone applies; other keys are dropped with a warning', async () => {
    const { plugin, issues } = await load({
      '.claude-plugin/plugin.json': manifest({
        agent: 'reviewer',
        subagentStatusLine: { type: 'command', command: 'jq .tasks' },
        model: 'opus',
      }),
    });
    // subagentStatusLine was removed in 2.1.323 (spec §12.6 item 1, J12.17): dropped like any key.
    expect(plugin?.settings).toEqual({ agent: 'reviewer' });
    expect(issues.map((found) => found.severity)).toEqual(['warning', 'warning']);
  });

  test('a settings.json that is not JSON is a warning and the manifest settings still apply (J12.10)', async () => {
    const { plugin, issues } = await load({
      '.claude-plugin/plugin.json': manifest({ agent: 'from-manifest' }),
      'settings.json': '{',
    });
    expect(plugin?.settings).toEqual({ agent: 'from-manifest' });
    expect(issues).toEqual([
      {
        severity: 'warning',
        path: 'settings.json',
        message: expect.stringMatching(/^settings\.json is not valid JSON: /),
      },
    ]);
  });

  test('when settings.json wins, the ignored manifest settings stays silent, a dropped key included', async () => {
    const { plugin, issues } = await load({
      '.claude-plugin/plugin.json': manifest({ agent: 'from-manifest', model: 'opus' }),
      'settings.json': json({ agent: 'from-file' }),
    });
    expect(plugin?.settings).toEqual({ agent: 'from-file' });
    expect(issues).toEqual([]);
  });
});

spec('manifest.field.userConfig', () => {
  test('the plugin carries the options the manifest declares', async () => {
    const option = {
      type: 'string',
      title: 'Token',
      description: 'API token',
      sensitive: true,
    } as const;
    const { plugin } = await load({
      '.claude-plugin/plugin.json': json({ name: 'p', userConfig: { api_token: option } }),
    });
    expect(plugin?.userConfig).toEqual({ api_token: option });
  });
});

describe('what a load reports (judgments)', () => {
  test('a path that does not exist skips that component and the rest loads', async () => {
    const { plugin, issues } = await load({
      '.claude-plugin/plugin.json': json({
        name: 'p',
        commands: ['./nope.md'],
        agents: ['./agents/a.md'],
      }),
      'agents/a.md': 'A.',
    });
    expect(plugin?.commands).toEqual([]);
    expect(namesOf(plugin?.agents ?? [])).toEqual(['a']);
    expect(messagesOf(issues)).toEqual(['commands path not found: ./nope.md']);
  });

  test('a command defined inline in the manifest carries its text and its entry', async () => {
    const { plugin } = await load({
      '.claude-plugin/plugin.json': json({
        name: 'my-plugin',
        commands: {
          about: { content: 'Explain what this plugin provides.', description: 'About' },
        },
      }),
    });
    expect(plugin?.commands).toEqual([
      {
        path: undefined,
        name: 'about',
        qualifiedName: 'my-plugin:about',
        text: 'Explain what this plugin provides.',
        entry: { content: 'Explain what this plugin provides.', description: 'About' },
      },
    ]);
  });

  test('the abort signal reaches every read of the files port', async () => {
    const controller = new AbortController();
    const seen: (AbortSignal | undefined)[] = [];
    const files = memoryFiles({ 'commands/a.md': 'A.' });
    const watching = {
      ...files,
      readText: (path: string, signal?: AbortSignal) => {
        seen.push(signal);
        return files.readText(path, signal);
      },
    };
    await loadPlugin({ files: watching, fallbackName: 'p', signal: controller.signal });
    expect(seen.length).toBeGreaterThan(0);
    expect(seen.every((signal) => signal === controller.signal)).toBe(true);
  });
});

describe('judgment J2.9: the list note while a plugin sets agent', () => {
  test('a plugin that sets agent loads with the plugin-agent-setting-deprecated note', async () => {
    const loaded = await load({ '.claude-plugin/plugin.json': manifest({ agent: 'reviewer' }) });
    expect(loaded.notes).toEqual([
      {
        type: 'plugin-agent-setting-deprecated',
        message:
          "A plugin's `agent` setting is deprecated and will stop applying in a future release.",
      },
    ]);
  });

  test('a plugin without a valid agent has no such note', async () => {
    const loaded = await load({ '.claude-plugin/plugin.json': manifest({ agent: 5 }) });
    expect(loaded.notes).toBeUndefined();
  });
});
