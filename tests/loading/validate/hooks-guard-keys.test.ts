/**
 * The top-level guard keys of a plugin's `hooks/hooks.json` (Agent Plugins Extended §7.2.1.1,
 * Judgment J7.20): oracle 2.1.325 refuses a `PreToolUse` or `PermissionRequest` key outside the
 * `hooks` object that holds anything but `null` or an empty list, with or without `hooks` beside it
 * (spec/judgments.md). A judgment, so outside `spec(...)`.
 */
import { expect, test } from 'bun:test';
import { validatePlugin } from '../../../src/index.ts';
import { memoryFiles } from '../../support/ports.ts';

const MANIFEST = JSON.stringify({
  name: 'demo',
  version: '1.0.0',
  description: 'Demo',
  author: { name: 'A' },
});
const hooksIssues = async (hooks: string) =>
  (
    await validatePlugin({
      files: memoryFiles({ '.claude-plugin/plugin.json': MANIFEST, 'hooks/hooks.json': hooks }),
    })
  ).issues;
const GROUP = [{ hooks: [{ type: 'command', command: 'x' }] }];

test('a guard key beside a hooks object is refused as one without it', async () => {
  for (const text of [
    JSON.stringify({ hooks: {}, PreToolUse: GROUP }),
    '{"hooks":{},"PreToolUse":{}}',
    '{"hooks":{},"PermissionRequest":5}',
  ]) {
    const [issue] = await hooksIssues(text);
    expect(issue?.message).toStartWith('PreToolUse/PermissionRequest is declared at the top level');
  }
});

test('null or an empty list there draws nothing', async () => {
  expect(await hooksIssues('{"hooks":{},"PreToolUse":null,"PermissionRequest":[]}')).toEqual([]);
});
