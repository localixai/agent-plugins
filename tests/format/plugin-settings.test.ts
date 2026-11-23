/**
 * Spec §12.6 item 1 ("Only `agent` takes effect. Every other key MUST be dropped at load ... An
 * `agent` that is not a string is dropped as well"); `subagentStatusLine` removed in 2.1.323
 * (Judgment J12.17).
 *
 * Applying the defaults (the lowest settings layer) is the host's; these tests prove which keys the kit
 * hands over.
 */
import { describe, expect, test } from 'bun:test';
import { hasAppliedSettings, readPluginSettings } from '../../src/entry/format.ts';

describe('the keys a plugin default keeps (spec §12.6 item 1; Judgments J1.3, J2.9)', () => {
  test('agent is the one key that takes effect', () => {
    const read = readPluginSettings({ agent: 'security-reviewer' }, 'settings.json');
    expect(read.settings).toEqual({ agent: 'security-reviewer' });
    expect(read.issues).toEqual([]);
  });

  test('subagentStatusLine is dropped like any other key, and a file holding only it applies nothing', () => {
    const read = readPluginSettings(
      { subagentStatusLine: { type: 'command', command: 'jq .tasks' } },
      'settings.json',
    );
    expect(read.settings).toEqual({});
    expect(hasAppliedSettings(read)).toBe(false);
    expect(read.issues.map((found) => [found.severity, found.path])).toEqual([
      ['warning', 'settings.json.subagentStatusLine'],
    ]);
  });

  test('every other key is dropped, each with a warning naming it', () => {
    const read = readPluginSettings(
      { agent: 'reviewer', model: 'opus', permissions: { allow: ['Bash'] }, env: { A: '1' } },
      'settings.json',
    );
    expect(read.settings).toEqual({ agent: 'reviewer' });
    expect(read.issues.map((found) => [found.severity, found.path])).toEqual([
      ['warning', 'settings.json.model'],
      ['warning', 'settings.json.permissions'],
      ['warning', 'settings.json.env'],
    ]);
  });

  test('a file with no supported key applies nothing', () => {
    const read = readPluginSettings({ model: 'opus' }, 'settings.json');
    expect(read.settings).toEqual({});
    expect(hasAppliedSettings(read)).toBe(false);
    expect(hasAppliedSettings(readPluginSettings({ agent: 'a' }, 'settings.json'))).toBe(true);
  });
});

describe('settings values (judgments)', () => {
  test('an agent that is not a string is dropped with a warning', () => {
    const read = readPluginSettings({ agent: 3 }, 'settings');
    expect(read.settings).toEqual({});
    expect(read.issues.map((found) => found.severity)).toEqual(['warning']);
  });

  test('settings that are not an object are a warning naming where they came from (J12.10)', () => {
    for (const raw of [null, [], 'agent']) {
      const read = readPluginSettings(raw, 'settings.json');
      expect(read.settings).toEqual({});
      expect(read.issues).toEqual([
        {
          severity: 'warning',
          path: 'settings.json',
          message: 'settings.json must be a JSON object of settings',
        },
      ]);
    }
  });
});
