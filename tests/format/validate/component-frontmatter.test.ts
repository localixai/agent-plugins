/**
 * The frontmatter findings of `claude plugin validate` on a skill, command or agent file (spec §4.10
 * and §5.4.4: oracle 2.1.321 corpus and 2.1.325). Wording, severity and which kind is checked are
 * the oracle's, so tested outside spec().
 */
import { describe, expect, test } from 'bun:test';
import { frontmatterFindings } from '../../../src/format/validate/component-frontmatter.ts';

const block = (yaml: string) => `---\ndescription: d\n${yaml}\n---\nBody.`;
const of = (kind: 'skill' | 'command' | 'agent', yaml: string) =>
  frontmatterFindings(kind, block(yaml)).map((found) => [
    found.severity,
    found.path,
    found.message,
  ]);

describe('skills and commands (§4.10)', () => {
  test('a list or map description is an error; a number is not reported', () => {
    const dropped = (type: string) => [
      'error',
      'description',
      `description must be a string, got ${type}. At runtime this value is dropped.`,
    ];
    expect(of('skill', '').length).toBe(0);
    expect(frontmatterFindings('skill', '---\ndescription: [a]\n---\nB')[0]?.message).toBe(
      dropped('array')[2],
    );
    expect(frontmatterFindings('command', '---\ndescription: {a: 1}\n---\nB')[0]?.message).toBe(
      dropped('object')[2],
    );
    expect(frontmatterFindings('skill', '---\ndescription: 5\n---\nB')).toEqual([]);
  });

  test('a list name is reported as object, like a map', () => {
    expect(of('skill', 'name: [a, b]')).toEqual([
      ['error', 'name', 'name must be a string, got object.'],
    ]);
    expect(of('command', 'name: 5')).toEqual([
      ['error', 'name', 'name must be a string, got number.'],
    ]);
  });

  test('allowed-tools: a wrong type, and a list with an item that is not text', () => {
    expect(of('skill', 'allowed-tools: 5')).toEqual([
      ['error', 'allowed-tools', 'allowed-tools must be a string or array of strings, got number.'],
    ]);
    expect(of('command', 'allowed-tools: [Read, 5]')).toEqual([
      ['error', 'allowed-tools', 'allowed-tools array must contain only strings.'],
    ]);
  });

  test('shell: not text, or not one of the two shells in any case', () => {
    expect(of('skill', 'shell: 5')).toEqual([
      ['error', 'shell', 'shell must be a string, got number.'],
    ]);
    expect(of('skill', 'shell: zsh')).toEqual([
      ['error', 'shell', "shell must be 'bash' or 'powershell', got 'zsh'."],
    ]);
    expect(of('skill', 'shell: Bash')).toEqual([]);
    expect(of('command', 'shell: PowerShell')).toEqual([]);
  });

  test('metadata that is not a mapping is a warning for a command too', () => {
    const warn = [
      'warning',
      'metadata',
      "'metadata' must be a mapping (key: value pairs); got string. It is dropped at load time.",
    ];
    expect(of('skill', 'metadata: text')).toEqual([warn]);
    expect(of('command', 'metadata: text')).toEqual([warn]);
  });
});

describe('agents (§5.4.4): name and description only', () => {
  test('the other fields pass silently', () => {
    expect(of('agent', 'allowed-tools: 5\nshell: zsh\nmetadata: text')).toEqual([]);
    expect(of('agent', 'name: [a]')).toEqual([
      ['error', 'name', 'name must be a string, got object.'],
    ]);
  });
});
