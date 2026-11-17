/**
 * permissions.md § `Write(path)`, `Glob(path)`, and `Grep(path)` rules don't match and § Tool name
 * wildcards; errors.md § Is not matched by file permission checks and § Matches no tool. Spec
 * 1.1.0 §13.2.1 item 3 and §13.5.2 (Judgment J13.33): a library returns each warning as a value,
 * the rule as written, its behaviour, its source, its kind and, for `never matches`, the rule to
 * use instead; the caller writes the row and the stderr line from it.
 */
import { describe, expect, test } from 'bun:test';
import type { PermissionBehavior } from '../../src/protocol/official.ts';
import { DOCUMENTED_TOOLS } from '../../src/protocol/tool-names.ts';
import { ruleIssues, ruleWarnings, type RuleSource } from '../../src/rules/index.ts';
import { parsed } from '../support/rule-context.ts';
import { spec } from '../support/spec.ts';

const known: ReadonlySet<string> = new Set(DOCUMENTED_TOOLS);
const PROJECT: RuleSource = { kind: 'project' };
const warningsOf = (text: string, behavior: PermissionBehavior, source: RuleSource = PROJECT) =>
  ruleWarnings([parsed(text, behavior, source)], known);

spec("permissions.behavior.Write(path), Glob(path), and Grep(path) rules don't match", () => {
  test('a Write path rule is kept and returned as a never-matches value with its replacement', () => {
    expect(warningsOf('Write(docs/**)', 'deny')).toEqual([
      {
        rule: 'Write(docs/**)',
        behavior: 'deny',
        source: PROJECT,
        kind: 'never matches',
        replacement: 'Edit(docs/**)',
        use: 'Edit',
      },
    ]);
  });

  test('NotebookEdit and MultiEdit use Edit, Glob uses Read', () => {
    expect(warningsOf('NotebookEdit(a/*.ipynb)', 'ask')[0]?.replacement).toBe('Edit(a/*.ipynb)');
    expect(warningsOf('MultiEdit(src/**)', 'deny').at(-1)?.replacement).toBe('Edit(src/**)');
    expect(warningsOf('Glob(docs/**)', 'deny')[0]?.replacement).toBe('Read(docs/**)');
  });

  test('an allow rule is warned about too (it only gets no row)', () => {
    expect(warningsOf('Write(docs/**)', 'allow')[0]?.kind).toBe('never matches');
  });

  test('every source warns: managed, a --settings file, --disallowedTools', () => {
    for (const source of [
      { kind: 'managed' },
      { kind: 'flag-settings', file: '/tmp/s.json' },
      { kind: 'cli' },
    ] satisfies RuleSource[]) {
      expect(warningsOf('Write(docs/**)', 'deny', source)[0]?.source).toEqual(source);
    }
  });

  test('--allowedTools accepts a Glob rule silently, not a Write rule', () => {
    expect(warningsOf('Glob(docs/**)', 'allow', { kind: 'cli' })).toEqual([]);
    expect(warningsOf('Write(docs/**)', 'allow', { kind: 'cli' })).toHaveLength(1);
    expect(warningsOf('Glob(docs/**)', 'deny', { kind: 'cli' })).toHaveLength(1);
  });

  test('a bare name, Tool(*) and a Grep path rule draw no warning', () => {
    for (const text of ['Write', 'Glob', 'Write(*)', 'Grep(docs/**)']) {
      expect([text, warningsOf(text, 'deny')]).toEqual([text, []]);
    }
  });

  test('the stderr form names the source in parentheses, managed as managed policy settings', () => {
    expect(ruleIssues([parsed('Write(docs/**)', 'deny')], known)[0]?.message).toBe(
      'Permission deny rule (.claude/settings.json): Write(docs/**) is not matched by file permission checks — only Edit(path) rules are. Use Edit(docs/**) instead (Edit rules cover all file-editing tools).',
    );
    const managed = ruleIssues([parsed('Glob(x/**)', 'ask', { kind: 'managed' })], known);
    expect(managed[0]?.message).toStartWith('Permission ask rule (managed policy settings): ');
    const flag = ruleIssues([parsed('Write(x)', 'deny', { kind: 'cli' })], known);
    expect(flag[0]?.message).toStartWith('Permission deny rule (--disallowed-tools): ');
  });
});

spec('permissions.behavior.Tool name wildcards', () => {
  test('a rule that matches no tool is a value with the kind matches no tool', () => {
    expect(warningsOf('Bsh(rm *)', 'deny', { kind: 'user' })).toEqual([
      { rule: 'Bsh(rm *)', behavior: 'deny', source: { kind: 'user' }, kind: 'matches no tool' },
    ]);
    expect(warningsOf('Bsh', 'allow')).toEqual([]);
  });
});

describe('judgment: MultiEdit is not a known tool (Agent Plugins Extended J13.34)', () => {
  test('a bare MultiEdit rule matches no tool; a path rule also draws the file-permission warning', () => {
    expect(warningsOf('MultiEdit', 'deny').map((found) => found.kind)).toEqual(['matches no tool']);
    expect(warningsOf('MultiEdit(docs/**)', 'deny').map((found) => found.kind)).toEqual([
      'matches no tool',
      'never matches',
    ]);
  });

  test('the TaskOutput exemption does not reach it', () => {
    expect(warningsOf('TaskOutput', 'deny')).toEqual([]);
    expect(warningsOf('MultiEdit', 'ask')).toHaveLength(1);
  });
});

describe('ruleWarnings: order', () => {
  test('one value per warning, in rule order', () => {
    const rules = [parsed('Bsh', 'deny'), parsed('Read', 'deny'), parsed('Glob(a)', 'ask')];
    expect(ruleWarnings(rules, known).map((found) => found.kind)).toEqual([
      'matches no tool',
      'never matches',
    ]);
  });
});
