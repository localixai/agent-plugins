/**
 * `convertPluginPackage`, the conversion of a plugin root through the files port (Agent Plugins
 * Extended §18.4.1): the form decided first (§1.3), nothing written, the source form never touched
 * (item 4: a file already there stops it), and the output judged by the target before a host writes
 * it (item 3). What the source's own loader skipped is reported, so nothing is lost silently (item 1).
 * Outside `spec(...)`: chapter 18 has no requirement ids.
 */
import { describe, expect, test } from 'bun:test';
import { CONVERSION_OUTCOMES } from '../../src/entry/core.ts';
import { CLAUDE_HALF, convertTree, CORE_HALF } from '../support/release-guard.ts';

const json = (value: unknown) => JSON.stringify(value);
const schema = (document: string) =>
  `https://agent-plugins.org/schemas/1.1.0/${document}.schema.json`;
const CORE = json({ $schema: schema('plugin'), name: 'acme-tools' });

const outcomes = (notes: readonly { readonly outcome: string; readonly path: string }[]) =>
  notes.map(({ outcome, path }) => `${outcome} ${path}`);

describe('§18.4.1: converting a plugin root', () => {
  test('the outcomes are a closed list', () => {
    expect(CONVERSION_OUTCOMES).toEqual(['stopped', 'not-converted', 'reported']);
  });

  test('a dual package is not converted: both forms are there', async () => {
    const converted = await convertTree({ ...CORE_HALF, ...CLAUDE_HALF });
    expect(converted).toMatchObject({ stopped: true, files: [] });
  });

  test('item 4: a file the output would overwrite stops the conversion', async () => {
    const converted = await convertTree({
      'plugin.json': CORE,
      '.mcp.json': '{}',
      'mcp.json': json({
        $schema: schema('mcp'),
        mcpServers: { a: { type: 'stdio', command: 'node' } },
      }),
    });
    expect(converted.stopped).toBe(true);
    expect(outcomes(converted.notes)).toEqual(['stopped .mcp.json']);
  });

  test('a source that does not load stops with its loader errors', async () => {
    const converted = await convertTree({
      '.claude-plugin/plugin.json': json({ name: 'x', version: 3 }),
    });
    expect(converted.stopped).toBe(true);
    expect(converted.notes.every((one) => one.outcome === 'stopped')).toBe(true);
  });

  test('item 1: an entry the source loader skips is reported', async () => {
    const converted = await convertTree({
      'plugin.json': CORE,
      'hooks.json': json({
        $schema: schema('hooks'),
        hooks: {
          Notification: [{ command: 'node' }],
          Stop: [{ command: 'node', args: ['x.mjs'] }],
        },
      }),
    });
    expect(converted.stopped).toBe(false);
    expect(outcomes(converted.notes)).toEqual(['not-converted hooks.Notification']);
  });

  test('item 3: what Claude Code would newly load from the core package is reported', async () => {
    const converted = await convertTree({
      'plugin.json': CORE,
      'commands/ship.md': '---\ndescription: Ship.\n---\nShip.\n',
      'agents/reviewer.md': '---\nname: reviewer\ndescription: Reviews.\n---\nReview.\n',
    });
    expect(outcomes(converted.notes)).toEqual([
      'reported commands/ship.md',
      'reported agents/reviewer.md',
    ]);
  });

  test('item 3: output claude plugin validate refuses stops the conversion', async () => {
    const converted = await convertTree({
      'plugin.json': CORE,
      'commands/broken.md': '---\nallowed-tools: 3\n---\nx\n',
    });
    expect(converted.stopped).toBe(true);
    expect(
      converted.notes.some(
        (one) => one.outcome === 'stopped' && one.message.startsWith('claude plugin validate:'),
      ),
    ).toBe(true);
  });
});
