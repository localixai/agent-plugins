/**
 * Spec §12.6, Plugin default settings, a deprecated feature: "Only `agent` takes effect"; the rules
 * the 2.1.321 documentation stated (File over manifest, User settings over plugin defaults, Two
 * plugins set the key, "`claude --debug` logs `overrides setting`") stay in force (Judgment J12.14),
 * so they are tested as judgments; the startup deprecation warning (§12.6 item 2). §12.7.1 `agent`:
 * "Any file; deprecated plugin default".
 */
import { describe, expect, test } from 'bun:test';
import { loadPlugin } from '../../../src/index.ts';
import { PLUGIN_AGENT_DEPRECATED } from '../../../src/format/plugin-settings.ts';

import { hooksFile, json } from '../../support/plugin-folder.ts';
import { memoryFiles } from '../../support/ports.ts';
import { viewOf } from '../../support/plugin-settings.ts';
import { spec } from '../../support/spec.ts';
import {
  pluginDefaultsOf,
  valueWithPluginDefaults,
} from '../../../src/settings/general/plugin-defaults.ts';

const load = async (tree: Record<string, string>) => {
  const { plugin } = await loadPlugin({ files: memoryFiles(tree), fallbackName: 'p' });
  return plugin?.settings ?? {};
};

describe('file over manifest (spec §12.6 item 3, Judgments J12.10, J12.14)', () => {
  const manifest = (settings: unknown) => ({
    '.claude-plugin/plugin.json': json({ name: 'p', settings }),
  });

  test('when both exist and settings.json sets a valid agent, settings.json applies and the manifest settings is ignored', async () => {
    const settings = await load({
      ...manifest({ agent: 'from-manifest' }),
      'settings.json': json({ agent: 'from-file' }),
    });
    expect(settings).toEqual({ agent: 'from-file' });
  });

  test('a settings.json with no supported key does not win: the manifest applies', async () => {
    const settings = await load({
      ...manifest({ agent: 'from-manifest' }),
      'settings.json': json({ model: 'opus' }),
    });
    expect(settings).toEqual({ agent: 'from-manifest' });
  });

  test('with only the manifest key, the inline object is the plugin defaults; with only the file, the file is', async () => {
    expect(await load(manifest({ agent: 'inline' }))).toEqual({ agent: 'inline' });
    expect(
      await load({ 'settings.json': json({ agent: 'file' }), 'hooks/hooks.json': hooksFile('x') }),
    ).toEqual({
      agent: 'file',
    });
  });

  test('a settings.json that holds only subagentStatusLine sets no supported key: the manifest applies', async () => {
    const settings = await load({
      ...manifest({ agent: 'from-manifest' }),
      'settings.json': json({ subagentStatusLine: { type: 'command', command: 'f' } }),
    });
    expect(settings).toEqual({ agent: 'from-manifest' });
  });

  test('subagentStatusLine never applies from a plugin, in the file or the manifest', async () => {
    const line = { type: 'command', command: 'm' };
    expect(await load(manifest({ subagentStatusLine: line }))).toEqual({});
    expect(
      await load({
        'settings.json': json({ subagentStatusLine: line }),
        'hooks/hooks.json': hooksFile('x'),
      }),
    ).toEqual({});
  });
});

spec('settings.key.agent', () => {
  const defaults = pluginDefaultsOf([{ name: 'p', settings: { agent: 'plugin-agent' } }]);

  test('plugin defaults apply when no settings source sets the key, naming the plugin', () => {
    expect(valueWithPluginDefaults(viewOf(), 'agent', defaults)).toEqual({
      value: 'plugin-agent',
      plugin: { name: 'p', settings: { agent: 'plugin-agent' } },
    });
  });

  test('nothing sets the key: no value', () => {
    expect(valueWithPluginDefaults(viewOf(), 'agent', pluginDefaultsOf([]))).toBeUndefined();
  });

  test('a user agent in ~/.claude/settings.json overrides the plugin', () => {
    expect(valueWithPluginDefaults(viewOf(['user', { agent: 'mine' }]), 'agent', defaults)).toEqual(
      { value: 'mine' },
    );
  });

  test('plugin defaults are the lowest layer across every settings source', () => {
    for (const scope of ['user', 'project', 'local', 'managed', 'flag'] as const) {
      expect(
        valueWithPluginDefaults(viewOf([scope, { agent: scope }]), 'agent', defaults)?.value,
      ).toBe(scope);
    }
  });

  test('a key a source holds with an invalid value is dropped on read, so the plugin default shows through', () => {
    expect(valueWithPluginDefaults(viewOf(['user', { agent: 3 }]), 'agent', defaults)?.value).toBe(
      'plugin-agent',
    );
  });
});

describe('two plugins set the key (spec §12.6 item 5, Judgments J12.11, J12.12, J12.14)', () => {
  test('the value from the plugin loaded last applies', () => {
    const defaults = pluginDefaultsOf([
      { name: 'first', settings: { agent: 'a' } },
      { name: 'second', settings: { agent: 'b' } },
    ]);
    expect(defaults.settings).toEqual({ agent: 'b' });
    expect(valueWithPluginDefaults(viewOf(), 'agent', defaults)?.plugin?.name).toBe('second');
  });

  describe('claude --debug logs overrides setting', () => {
    test('one line per overridden key, naming both plugins', () => {
      const defaults = pluginDefaultsOf([
        { name: 'first', settings: { agent: 'a' } },
        { name: 'second', settings: { agent: 'b' } },
        { name: 'third', settings: { agent: 'c' } },
      ]);
      expect(defaults.debug).toHaveLength(2);
      expect(defaults.debug[0]).toBe(
        'Plugin "second" overrides setting "agent" set by plugin "first"',
      );
      expect(defaults.debug[1]).toBe(
        'Plugin "third" overrides setting "agent" set by plugin "second"',
      );
    });

    test('no line when no plugin overrides another', () => {
      const defaults = pluginDefaultsOf([
        { name: 'first', settings: { agent: 'a' } },
        { name: 'second', settings: {} },
      ]);
      expect(defaults.debug).toEqual([]);
    });
  });

  test('a plugin that sets nothing changes nothing', () => {
    expect(pluginDefaultsOf([{ name: 'p', settings: {} }])).toEqual({
      settings: {},
      owners: {},
      debug: [],
      warnings: [],
    });
  });

  describe('the load order is the order given', () => {
    test('reversing the list reverses the winner', () => {
      const plugins = [
        { name: 'one', settings: { agent: '1' } },
        { name: 'two', settings: { agent: '2' } },
      ];
      expect(pluginDefaultsOf(plugins).settings.agent).toBe('2');
      expect(pluginDefaultsOf(plugins.toReversed()).settings.agent).toBe('1');
    });
  });
});

describe('the startup deprecation warning (spec §12.6 item 2, Judgment J12.14)', () => {
  test('one warning per enabled plugin that sets agent, in load order, with the validator text', () => {
    const first = { name: 'first', settings: { agent: 'a' } };
    const second = { name: 'second', settings: { agent: 'b' } };
    const defaults = pluginDefaultsOf([first, { name: 'none', settings: {} }, second]);
    expect(defaults.warnings).toEqual([
      { plugin: first, message: PLUGIN_AGENT_DEPRECATED },
      { plugin: second, message: PLUGIN_AGENT_DEPRECATED },
    ]);
    expect(PLUGIN_AGENT_DEPRECATED).toBe(
      "A plugin's `agent` setting is deprecated and will stop applying in a future release. Tell people to set `agent` in their own settings or pass --agent.",
    );
  });

  test('the warning is given even when a settings source overrides the plugin', () => {
    const defaults = pluginDefaultsOf([{ name: 'p', settings: { agent: 'plugin-agent' } }]);
    expect(valueWithPluginDefaults(viewOf(['user', { agent: 'mine' }]), 'agent', defaults)).toEqual(
      {
        value: 'mine',
      },
    );
    expect(defaults.warnings).toHaveLength(1);
  });
});
