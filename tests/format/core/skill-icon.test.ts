/**
 * A core package's skill files and icon candidates (Agent Plugins 1.1.0, `spec/1.1.0.md`): core §7.1
 * (a `SKILL.md` conforms to the Agent Skills specification, agentskills.io/specification,
 * "Frontmatter", or the skill is skipped) and §4.3.1 (icon candidates: lowercase `icon.<ext>`
 * directly in `assets/`, PNG always recognised). Judgment-style tests outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { checkCoreSkill, iconCandidates } from '../../../src/entry/core.ts';

const skill = (frontmatter: string) => `---\n${frontmatter}\n---\nBody\n`;

describe('core §7.1: Agent Skills conformance', () => {
  test('a skill with name and description conforms', () => {
    expect(checkCoreSkill(skill('name: deploy\ndescription: Deploys.'), 'deploy')).toEqual({
      problems: [],
      unknownFields: [],
    });
  });

  test('all six fields are accepted', () => {
    const text = skill(
      'name: pdf\ndescription: d\nlicense: MIT\ncompatibility: git\nmetadata:\n  a: "1"\nallowed-tools: Read Bash(git:*)',
    );
    expect(checkCoreSkill(text, 'pdf').problems).toEqual([]);
  });

  test('no frontmatter, no name, no description, or a name unlike the directory skips the skill', () => {
    expect(checkCoreSkill('Body only', 'x').problems).toEqual(['SKILL.md has no frontmatter']);
    expect(checkCoreSkill(skill('description: d'), 'x').problems).toEqual([
      '"name" is required and must be a string',
    ]);
    expect(checkCoreSkill(skill('name: x'), 'x').problems).toEqual([
      '"description" is required: a string of 1 to 1024 characters',
    ]);
    expect(checkCoreSkill(skill('name: other\ndescription: d'), 'x').problems).toEqual([
      '"name" "other" must match its directory "x"',
    ]);
  });

  test('the name rules: lowercase letters, digits, single hyphens, at most 64', () => {
    for (const name of ['PDF', '-pdf', 'pdf-', 'pdf--x', 'a.b', 'x'.repeat(65)]) {
      expect(checkCoreSkill(skill(`name: "${name}"\ndescription: d`), name).problems).toHaveLength(
        1,
      );
    }
  });

  test('description up to 1024, compatibility up to 500, metadata of strings', () => {
    expect(
      checkCoreSkill(skill(`name: x\ndescription: "${'d'.repeat(1025)}"`), 'x').problems,
    ).toHaveLength(1);
    expect(
      checkCoreSkill(skill(`name: x\ndescription: d\ncompatibility: "${'c'.repeat(501)}"`), 'x')
        .problems,
    ).toHaveLength(1);
    expect(
      checkCoreSkill(skill('name: x\ndescription: d\nmetadata:\n  n: 1'), 'x').problems,
    ).toHaveLength(1);
  });

  test('judgment: a field outside Agent Skills is reported, not a reason to skip', () => {
    expect(checkCoreSkill(skill('name: x\ndescription: d\nargument-hint: "[a]"'), 'x')).toEqual({
      problems: [],
      unknownFields: ['argument-hint'],
    });
  });
});

const file = (name: string) => ({ name, isFile: true, isDirectory: false });

describe('core §4.3.1: icon candidates', () => {
  test('only lowercase icon.<ext> files are candidates, PNG first', () => {
    const entries = [
      file('icon.svg'),
      file('icon.png'),
      file('Icon.png'),
      file('icon.PNG'),
      file('icon.png.backup'),
      { name: 'icons', isFile: false, isDirectory: true },
    ];
    expect(iconCandidates(entries, ['svg'])).toEqual(['assets/icon.png', 'assets/icon.svg']);
  });

  test('a format the host does not recognise is no candidate; PNG always is', () => {
    expect(iconCandidates([file('icon.webp'), file('icon.png')])).toEqual(['assets/icon.png']);
  });
});
