/**
 * Spec §12.6 item 2 and Judgments J1.3, J2.9: `claude plugin validate` warns, without failing, about
 * `agent` and `subagentStatusLine` in a plugin's defaults, in the manifest's `settings` and in a root
 * `settings.json`, whatever the value; nothing for another key, a wrong type or a file that does not
 * parse. The ids `manifest.field.settings`, `manifest.rule.settings` and `layout.Settings` are gone
 * from the 2.1.330 requirements, so these are judgment tests.
 */
import { describe, expect, test } from 'bun:test';
import { validatePlugin } from '../../../src/loading/index.ts';
import { json } from '../../support/plugin-folder.ts';
import { memoryFiles } from '../../support/ports.ts';

const AGENT =
  "A plugin's `agent` setting is deprecated and will stop applying in a future release. Tell people to set `agent` in their own settings or pass --agent.";
const STATUS =
  'subagentStatusLine is deprecated in favor of mods and will stop working in a future release.';

const MANIFEST = { name: 'p', version: '1.0.0', description: 'd', author: { name: 'a' } };

const validate = async (tree: Record<string, string>) =>
  (await validatePlugin({ files: memoryFiles(tree) })).issues;

describe('judgment J2.9: deprecated plugin defaults in the validator', () => {
  test('the manifest settings: agent and subagentStatusLine warn, other keys say nothing', async () => {
    const issues = await validate({
      '.claude-plugin/plugin.json': json({
        ...MANIFEST,
        settings: { agent: 5, subagentStatusLine: {}, model: 'opus' },
      }),
    });
    expect(issues).toEqual([
      {
        severity: 'warning',
        file: '.claude-plugin/plugin.json',
        path: 'settings.agent',
        message: AGENT,
      },
      {
        severity: 'warning',
        file: '.claude-plugin/plugin.json',
        path: 'root',
        message: `settings.${STATUS}`,
      },
    ]);
  });

  test('a root settings.json with agent warns under settings.json', async () => {
    const issues = await validate({
      '.claude-plugin/plugin.json': json(MANIFEST),
      'settings.json': json({ agent: 'reviewer', subagentStatusLine: 1 }),
    });
    expect(issues).toEqual([
      { severity: 'warning', file: 'settings.json', path: 'agent', message: AGENT },
      { severity: 'warning', file: 'settings.json', path: 'root', message: STATUS },
    ]);
  });

  test('a settings.json that does not parse, or holds other keys only, draws nothing', async () => {
    for (const text of ['{bad', '[1]', json({ model: 'opus', permissions: {} })]) {
      expect(
        await validate({ '.claude-plugin/plugin.json': json(MANIFEST), 'settings.json': text }),
      ).toEqual([]);
    }
  });

  test('a manifest settings that is not an object fails validation', async () => {
    const issues = await validate({
      '.claude-plugin/plugin.json': json({ ...MANIFEST, settings: 'str' }),
    });
    expect(issues).toContainEqual(expect.objectContaining({ severity: 'error', path: 'settings' }));
  });
});
