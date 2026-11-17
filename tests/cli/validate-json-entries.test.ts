/**
 * plugins/cli-reference.md § plugin validate, "Output and exit codes": the docs name the `--json`
 * arrays and the four `gatingHooks` fields, not the entries. Agent Plugins Extended J17.15 (oracle
 * 2.1.325): `notes` are plain strings, a `gatingHooks` entry is `{ module, pattern, hook, hasCatch }`
 * with `pattern` the string given to `on`. Judgments, so outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { validationJson } from '../../src/cli/index.ts';

describe('plugin validate --json: gating hooks (judgment)', () => {
  test('a file whose only report is a gating hook still has a result', () => {
    const gate = {
      module: './register.js',
      pattern: 'tool.call',
      hook: 'tool.call',
      hasCatch: true,
    };
    const json = validationJson({
      target: 'plugin',
      path: '/w',
      strict: false,
      issues: [],
      gatingHooks: { 'hooks/hooks.json': [gate] },
    });
    expect(json.contents).toEqual([
      { file: 'hooks/hooks.json', errors: [], warnings: [], notes: [], gatingHooks: [gate] },
    ]);
  });

  test('notes are plain strings, by the file they belong to (Agent Plugins Extended J17.15)', () => {
    const line = './register.js hooks: tool.call{tool=Bash}, tool.call';
    const json = validationJson({
      target: 'plugin',
      path: '/w',
      strict: false,
      issues: [],
      notes: { 'mods/m/mod.json': [line] },
    });
    expect(json.contents).toEqual([
      { file: 'mods/m/mod.json', errors: [], warnings: [], notes: [line], gatingHooks: [] },
    ]);
  });
});
