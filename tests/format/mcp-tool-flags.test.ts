/**
 * Agent Plugins Extended §8.8 (mcp.md, "Tool input schemas with a root-level combinator", "Tools with
 * invalid input schemas"; 2.1.328 and later): with feature-flag fetching off, the root-level rewrite
 * runs and the schema checks exclude a tool; only while fetching is on and the flags have not arrived
 * yet is a combinator tool skipped and an invalid one sent with the log line.
 */
import { expect, test } from 'bun:test';
import { mcpToolPolicy, prepareMcpTool } from '../../src/entry/format.ts';
import { spec } from '../support/spec.ts';

const combinator = {
  name: 'lookup',
  inputSchema: { anyOf: [{ properties: { a: { type: 'string' } }, required: ['a'] }] },
};
const invalid = { name: 'lookup', inputSchema: { type: 'object', properties: { 'a b': {} } } };

spec('mcp.behavior.Tools with invalid input schemas', () => {
  test('with flag fetching off the checks apply: the tool is excluded', () => {
    expect(prepareMcpTool(invalid, mcpToolPolicy('off')).sent).toBe(false);
    expect(prepareMcpTool(invalid, mcpToolPolicy('arrived')).sent).toBe(false);
  });

  test('only while the flags have not arrived yet is the tool sent, the log naming the API', () => {
    const pending = prepareMcpTool(invalid, mcpToolPolicy('pending'));
    expect(pending.sent).toBe(true);
    expect(pending.log.join('\n')).toContain('input schema would be rejected by the Anthropic API');
  });
});

spec('mcp.behavior.Tool input schemas with a root-level combinator', () => {
  test('with flag fetching off the rewrite runs; while the flags are pending the tool is skipped', () => {
    expect(prepareMcpTool(combinator, mcpToolPolicy('off')).sent).toBe(true);
    expect(prepareMcpTool(combinator, mcpToolPolicy('arrived')).sent).toBe(true);
    expect(prepareMcpTool(combinator, mcpToolPolicy('pending')).sent).toBe(false);
  });
});
