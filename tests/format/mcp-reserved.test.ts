/**
 * mcp.md, the reserved server names (Agent Plugins Extended §8.5 item 5; errors.md, "MCP server name
 * is reserved"): a server of the user's own configuration named `workspace`, `claude-in-chrome`,
 * `computer-use`, `Claude Preview` or `Claude Browser` is skipped with the message; a plugin's server
 * key is not checked (J8.12).
 */
import { expect, test } from 'bun:test';
import { RESERVED_MCP_SERVER_NAMES } from '../../src/format/mcp-reserved.ts';
import { loadPlugin } from '../../src/loading/index.ts';
import { claudeJsonServers, projectFileServers } from '../../src/settings/mcp/candidates.ts';
import { memoryFiles } from '../support/ports.ts';
import { spec } from '../support/spec.ts';

const server = { command: 'node', args: ['s.js'] };
const servers = Object.fromEntries(
  [...RESERVED_MCP_SERVER_NAMES, 'mine'].map((name) => [name, server]),
);

spec('cli.errors.MCP server name is reserved', () => {
  test('a project .mcp.json server under a reserved name does not load, with the message', () => {
    const read = projectFileServers({ mcpServers: servers });
    expect(read.candidates.map((one) => one.name)).toEqual(['mine']);
    expect(read.issues.map((issue) => [issue.severity, issue.message])).toEqual(
      RESERVED_MCP_SERVER_NAMES.map((name) => [
        'warning',
        `"${name}" is a reserved MCP server name and was not loaded`,
      ]),
    );
  });

  test('so do the user and local servers of ~/.claude.json', () => {
    const state = {
      mcpServers: { workspace: server },
      projects: { '/p': { mcpServers: servers } },
    };
    const read = claudeJsonServers(state, '/p');
    expect(read.candidates.map((one) => [one.scope, one.name])).toEqual([['local', 'mine']]);
    expect(read.issues).toHaveLength(RESERVED_MCP_SERVER_NAMES.length + 1);
  });

  test('the match is exact; a plugin server key workspace loads', async () => {
    expect(projectFileServers({ mcpServers: { Workspace: server } }).issues).toEqual([]);
    const { plugin } = await loadPlugin({
      files: memoryFiles({
        '.claude-plugin/plugin.json': JSON.stringify({ name: 'p' }),
        '.mcp.json': JSON.stringify({ mcpServers: { workspace: server } }),
      }),
    });
    expect(Object.keys(plugin?.mcpServers ?? {})).toEqual(['workspace']);
  });
});
