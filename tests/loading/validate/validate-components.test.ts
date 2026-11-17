/**
 * `claude plugin validate`: a project `.claude/agents/` folder without a manifest (J2.4) and the
 * many-tools unknown-field family (J2.7), oracle observations recorded in spec/judgments.md.
 */
import { describe, expect, test } from 'bun:test';
import { validatePlugin } from '../../../src/index.ts';
import { memoryFiles } from '../../support/ports.ts';

type Tree = Parameters<typeof memoryFiles>[0];

const COMPLETE = { name: 'demo', version: '1.0.0', description: 'Demo', author: { name: 'A' } };
const manifest = (extra: Record<string, unknown> = {}): string =>
  JSON.stringify({ ...COMPLETE, ...extra });
const run = (tree: Tree) => validatePlugin({ files: memoryFiles(tree) });

describe('a project .claude/agents folder (J2.4, oracle 2.1.319 and 2.1.322)', () => {
  test('an agent whose YAML does not parse is an error: it does not load at all', async () => {
    const report = await run({
      '.claude/agents/a.md': '---\nname: a\ndescription: "unclosed\n---\n',
    });
    expect(report.issues.map((issue) => [issue.severity, issue.file, issue.path])).toEqual([
      ['error', '.claude/agents/a.md', 'frontmatter'],
    ]);
    expect(report.issues[0]?.message).toEndWith(
      'At runtime this agent does not load at all — with no frontmatter name it is treated as a co-located reference document and skipped.',
    );
  });

  test('an agent without a description draws the description warning', async () => {
    const report = await run({ '.claude/agents/a.md': '---\nname: a\n---\nBody' });
    expect(report.issues.map((issue) => [issue.severity, issue.path, issue.message])).toEqual([
      [
        'warning',
        'description',
        'No description in frontmatter. A description helps users and Claude understand when to use this agent.',
      ],
    ]);
  });
});

describe('unknown fields', () => {
  test('support and privacy_policy draw the many-tools warning (J2.7, oracle 2.1.321)', async () => {
    const report = await run({
      '.claude-plugin/plugin.json': manifest({ support: 's', privacy_policy: 'p' }),
    });
    expect(report.issues.map((issue) => issue.message)).toEqual(
      ['support', 'privacy_policy'].map(
        (key) =>
          `Unknown field '${key}' (commonly seen in manifests across many tools). Claude Code ignores unrecognized fields at load time, so it's safe to keep.`,
      ),
    );
  });
});
