/**
 * A command and a skill of one plugin with the same command name (`commands/foo.md` and
 * `skills/foo/SKILL.md` are both `/my-plugin:foo`): the docs are silent within a plugin, so this is
 * the judgment of spec/judgments.md (oracle 2.1.319, re-run on 2.1.325): the command is listed once,
 * with its description, and runs; the skill is not reachable by that name.
 */
import { describe, expect, test } from 'bun:test';
import { hostOf, loaded, SESSION, signal } from '../support/host.ts';

const page = (description: string, body: string): string =>
  ['---', `description: ${description}`, '---', body].join('\n');

describe('judgments (spec/judgments.md): a plugin command and skill of one name', () => {
  test('the command wins: one row with its description, and its body runs', async () => {
    const plugin = await loaded({
      '.claude-plugin/plugin.json': JSON.stringify({ name: 'my-plugin' }),
      'commands/foo.md': page('From the command', 'Command body.'),
      'skills/foo/SKILL.md': page('From the skill', 'Skill body.'),
    });
    const { host } = await hostOf({ plugins: [{ id: 'my-plugin@acme', plugin }] });
    const session = host.openSession(SESSION);
    const rows = session.skills().menu.filter((entry) => entry.name === 'my-plugin:foo');
    expect(rows).toHaveLength(1);
    expect(rows[0]?.description).toBe('From the command');
    const run = await session.expandSkill('my-plugin:foo', { signal: signal() });
    expect(run.ok && run.text).toContain('Command body.');
    expect(run.ok && run.text).not.toContain('Skill body.');
  });
});
