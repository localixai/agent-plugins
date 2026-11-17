/**
 * skills.md § Characters a skill name can't hold (the 256-character limit) and errors.md § Skill or
 * command name refused: a refused name a skill goes by keeps it from loading, a refused frontmatter
 * `name` is ignored, and each has one `[names]` debug-log line with hidden characters as codes.
 */
import { describe, expect, test } from 'bun:test';
import { blocksLoading } from '../../src/format/issues.ts';
import { resolveSkillName } from '../../src/format/skill-name.ts';
import type { SkillLayout } from '../../src/format/skill-types.ts';
import { spec } from '../support/spec.ts';

const project = (dirName: string): SkillLayout => ({ origin: { kind: 'project' }, dirName });
const plugin = (dirName: string): SkillLayout => ({
  origin: { kind: 'plugin', plugin: 'my-plugin' },
  dirName,
});

const errors = (layout: SkillLayout, declared?: string) =>
  resolveSkillName(layout, declared).issues.filter((found) => found.severity === 'error');

spec("skills.behavior.Characters a skill name can't hold", () => {
  test('a name over 256 characters is refused: a folder does not load, a frontmatter name is ignored', () => {
    // skills.md, "Characters a skill name can't hold"; errors.md: "or be over 256 characters".
    const long = 'a'.repeat(257);
    expect(errors(project(long))).toHaveLength(1);
    expect(errors(project('a'.repeat(256)))).toEqual([]);
    const named = resolveSkillName(project('ok'), long);
    expect(named).toMatchObject({ name: 'ok', command: 'ok' });
    expect(blocksLoading(named.issues)).toBe(false);
    expect(resolveSkillName(plugin('review'), long).command).toBe('my-plugin:review');
  });
});

spec('cli.errors.Skill or command name refused', () => {
  test('a refused name is reported as its [names] debug line, hidden characters spelled as codes', () => {
    // errors.md, "Skill or command name refused": `<name> is not offered`, `<name value> is not
    // shown for <skill>`, "spells out some hidden characters as codes, such as `\u{200B}`".
    const [ignored] = resolveSkillName(project('ok'), 'a\u200Bb').issues;
    expect(ignored).toMatchObject({
      severity: 'warning',
      path: 'name',
      message: '[names] a\\u{200B}b is not shown for ok',
    });
    const [refused] = resolveSkillName(project('de ploy '), undefined).issues;
    expect(refused).toMatchObject({
      severity: 'error',
      path: 'folder',
      message: '[names] de ploy\\u{20} is not offered',
    });
    expect(resolveSkillName(project('a<b'), undefined).issues[0]?.message).toBe(
      '[names] a<b is not offered',
    );
  });
});

describe('judgments (spec/judgments.md)', () => {
  test('a refused folder name still returns the name as written, for the host to report', () => {
    expect(resolveSkillName(project('a<b'), undefined)).toMatchObject({
      name: 'a<b',
      command: 'a<b',
    });
  });
});
