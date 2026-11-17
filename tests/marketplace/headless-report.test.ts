/**
 * headless.md § Fail CI when a plugin or MCP server doesn't load (`plugins`, `plugin_errors`, the
 * `path` of a failed `--plugin-dir` entry) and § Track plugin installs (`system/plugin_install`);
 * the SDK's `SDKSystemMessage.plugin_errors` comment (`inline[N]` / `synced[N]`, a Remote Control
 * worker omits the key). No requirement id covers these paragraphs, so the tests stand outside
 * `spec(...)` (spec/judgments.md).
 */
import { describe, expect, test } from 'bun:test';
import { initPluginFields, pluginInstallEvents } from '../../src/marketplace/headless-report.ts';

describe('system/init plugin fields', () => {
  test('a clean load lists the plugins and omits plugin_errors', () => {
    const fields = initPluginFields({
      loaded: [{ id: 'fmt@team', name: 'fmt', path: '/cache/fmt', version: '1.2.0' }],
    });
    expect(fields).toEqual({ plugins: [{ name: 'fmt', path: '/cache/fmt', version: '1.2.0' }] });
    expect('plugin_errors' in fields).toBe(false);
  });

  test('every failure is an entry; what did not load is absent from plugins', () => {
    const fields = initPluginFields({
      loaded: [
        {
          id: 'fmt@team',
          name: 'fmt',
          path: '/cache/fmt',
          errors: [{ type: 'hook-load-failed', message: 'bad hooks' }],
        },
      ],
      unloaded: [
        { id: 'lint@team', error: { type: 'dependency-unsatisfied', message: 'needs fmt ^2' } },
      ],
      failedDirectories: [
        {
          origin: 'inline',
          index: 1,
          path: '/work/missing',
          error: { type: 'path-not-found', message: 'not found' },
        },
      ],
    });
    expect(fields.plugins).toEqual([{ name: 'fmt', path: '/cache/fmt' }]);
    expect(fields.plugin_errors).toEqual([
      { plugin: 'fmt@team', type: 'hook-load-failed', message: 'bad hooks' },
      { plugin: 'lint@team', type: 'dependency-unsatisfied', message: 'needs fmt ^2' },
      { plugin: 'inline[1]', type: 'path-not-found', message: 'not found', path: '/work/missing' },
    ]);
  });

  test('a session persisted server-side always omits plugin_errors', () => {
    const fields = initPluginFields({
      loaded: [],
      unloaded: [{ id: 'a@m', error: { type: 'generic-error', message: 'x' } }],
      persistedRemotely: true,
    });
    expect(fields).toEqual({ plugins: [] });
  });
});

describe('system/plugin_install events', () => {
  test('started and completed bracket one event per marketplace', () => {
    let n = 0;
    const events = pluginInstallEvents(
      [{ name: 'team' }, { name: 'other', error: 'clone failed' }],
      { sessionId: 's1', uuid: () => `0000000${++n}-0000-0000-0000-000000000000` },
    );
    expect(events.map((event) => [event.status, event.name, event.error])).toEqual([
      ['started', undefined, undefined],
      ['installed', 'team', undefined],
      ['failed', 'other', 'clone failed'],
      ['completed', undefined, undefined],
    ]);
    expect(events.every((event) => event.type === 'system' && event.session_id === 's1')).toBe(
      true,
    );
    expect(new Set(events.map((event) => event.uuid)).size).toBe(4);
    expect('name' in (events[0] ?? {})).toBe(false);
  });
});
