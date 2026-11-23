/**
 * Spec §17.13.3 (plugins/cli-reference.md, "Validate a directory": "Plugin files in a marketplace
 * run") and Judgment J17.35: a marketplace run reads the hooks files of the plugins it lists by
 * relative path, reporting them as a plugin run does; their skill, agent and command files are not
 * read.
 */
import { expect, test } from 'bun:test';
import { validatePlugin } from '../../../src/loading/index.ts';
import { json } from '../../support/plugin-folder.ts';
import { memoryFiles } from '../../support/ports.ts';
import { spec } from '../../support/spec.ts';

const MARKETPLACE = json({
  name: 'mk',
  owner: { name: 'o' },
  plugins: [{ name: 'p', source: './p', description: 'd' }],
});

const validate = async (tree: Record<string, string>) =>
  (
    await validatePlugin({
      files: memoryFiles({ '.claude-plugin/marketplace.json': MARKETPLACE, ...tree }),
    })
  ).issues;

spec('cli.plugin.Validate a directory.Plugin files in a marketplace run', () => {
  test("a listed plugin's hooks/hooks.json that is not JSON fails the run", async () => {
    const issues = await validate({
      'p/.claude-plugin/plugin.json': json({ name: 'p' }),
      'p/hooks/hooks.json': '{"hooks": {',
    });
    expect(issues).toContainEqual(
      expect.objectContaining({ severity: 'error', file: 'p/hooks/hooks.json', path: 'json' }),
    );
  });

  test('a hooks file the entry manifest names is read too', async () => {
    const issues = await validate({
      'p/.claude-plugin/plugin.json': json({ name: 'p', hooks: './extra/hooks.json' }),
      'p/extra/hooks.json': '{bad',
    });
    expect(issues).toContainEqual(
      expect.objectContaining({ severity: 'error', file: 'p/extra/hooks.json', path: 'json' }),
    );
  });

  test("the listed plugin's skill with unparsable YAML is not reported", async () => {
    const issues = await validate({
      'p/.claude-plugin/plugin.json': json({ name: 'p' }),
      'p/skills/s/SKILL.md': '---\nname: [\n---\nbody',
    });
    expect(issues.filter((issue) => issue.file.startsWith('p/skills'))).toEqual([]);
  });
});
