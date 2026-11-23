/**
 * managed-mcp.md § "Exclusive control with managed-mcp.json" (Agent Plugins Extended §8.11.5): with the
 * file only its servers and those of `managedMcpServers` load, plugin servers included in what users
 * cannot use; mcp.md § "Scope hierarchy and precedence" (§8.11.1 item 2): in `claude -p`, Agent SDK and
 * VS Code sessions the claude.ai connectors rank above plugin-provided servers.
 */
import { expect, test } from 'bun:test';
import { EXCLUSIVE_ADD_ERROR } from '../../../src/settings/mcp/exclusive.ts';
import {
  configured,
  http,
  planOf,
  plannedLabels,
  plannedNames,
  provided,
} from '../../support/mcp-managed.ts';
import { candidate, labels } from '../../support/mcp-rank.ts';
import { rankMcpServers } from '../../../src/settings/mcp/index.ts';
import { spec } from '../../support/spec.ts';

const file = { servers: { github: http('https://api.githubcopilot.com/mcp/') } };
const search = provided('search', 'https://search.example.com/mcp');
const users = [
  configured('local', 'mine'),
  configured('project', 'shared'),
  configured('user', 'notes'),
  configured('plugin', 'plugin-search'),
  configured('connector', 'claude.ai Slack'),
];

spec('mcpSettings.managed.Exclusive control with managed-mcp.json', () => {
  test('with the file only its servers and the provided ones load: no local, project, user, plugin or connector server', () => {
    const plan = planOf({ managedFile: file, managedServers: [search], candidates: users });
    expect(plan.exclusive).toBe(true);
    expect(plannedLabels(plan).toSorted()).toEqual([
      'managedFile:github',
      'managedSettings:search',
    ]);
  });

  test('users cannot add servers while it is deployed', () => {
    expect(planOf({ managedFile: file }).addError).toBe(EXCLUSIVE_ADD_ERROR);
  });

  test('a session with connectors above plugins changes nothing under the file', () => {
    const plan = planOf({
      managedFile: file,
      candidates: users,
      session: { cloud: false, connectorsAbovePlugins: true },
    });
    expect(plannedLabels(plan)).toEqual(['managedFile:github']);
  });
});

spec('mcpSettings.behavior.Scope hierarchy and precedence', () => {
  test('without the file, claude -p, the SDK and VS Code rank a connector above a plugin server at its URL', () => {
    const same = http('https://docs.example.com/mcp');
    const candidates = [
      { scope: 'plugin', name: 'plugin:docs:docs', server: same },
      { scope: 'connector', name: 'claude.ai Docs', server: same },
    ] as const;
    const headless = planOf({
      managedFile: undefined,
      candidates,
      session: { cloud: false, connectorsAbovePlugins: true },
    });
    expect(plannedNames(headless)).toEqual(['claude.ai Docs']);
    expect(plannedNames(planOf({ managedFile: undefined, candidates }))).toEqual([
      'plugin:docs:docs',
    ]);
  });

  test('a plugin server and a connector at one URL: the plugin wins in a terminal session, the connector in claude -p, the SDK and VS Code', () => {
    const both = [
      candidate('connector', 'claude.ai Docs', http('https://mcp.example.com/mcp')),
      candidate('plugin', 'plugin:docs:docs', http('https://mcp.example.com/mcp')),
    ];
    expect(labels(rankMcpServers(both).loaded)).toEqual(['plugin:plugin:docs:docs']);
    const headless = rankMcpServers(both, { connectorsAbovePlugins: true });
    expect(labels(headless.loaded)).toEqual(['connector:claude.ai Docs']);
    expect(headless.hidden[0]?.candidate.scope).toBe('plugin');
  });

  test('connectors above plugins still rank below the user, project and local scopes', () => {
    const { loaded } = rankMcpServers(
      [
        candidate('connector', 'claude.ai Docs', http('https://mcp.example.com/mcp')),
        candidate('user', 'docs', http('https://mcp.example.com/mcp')),
      ],
      { connectorsAbovePlugins: true },
    );
    expect(labels(loaded)).toEqual(['user:docs']);
  });
});
