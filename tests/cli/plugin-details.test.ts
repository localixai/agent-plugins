/**
 * plugins/cli-reference.md § plugin details: "Claude Code prints the plugin's name, version,
 * description, and source, then these sections: `Component inventory`, `Projected token cost`,
 * `Per-component (rounded)` ... Omitted when the plugin has none"; a plugin that is not loaded
 * prints a message and exits `1`, and an `@builtin` id has its own message. Spec §17.9 item 1: each
 * kind "with its count and names"; the `Skills (1)  a` layout is oracle 2.1.325's (J17.11).
 */
import { describe, expect, test } from 'bun:test';
import { planPluginDetails, type PluginDetails } from '../../src/cli/index.ts';
import { spec } from '../support/spec.ts';
import { roundTokens } from '../../src/cli/plugin-details.ts';

const details: PluginDetails = {
  name: 'formatter',
  version: '1.2.0',
  description: 'Formats code',
  source: 'acme',
  inventory: {
    skills: ['format'],
    agents: [],
    hooks: ['PostToolUse'],
    mcpServers: ['docs'],
    lspServers: [],
  },
  projectedTokens: 1234,
  components: [{ kind: 'skill', name: 'format', alwaysOn: 87.4, onInvoke: 2310 }],
};

describe('plugin details (the page prose outside the three section ids)', () => {
  test('prints name, version, description and source first', () => {
    const out = planPluginDetails('formatter', details);
    expect(out.exitCode).toBe(0);
    expect(out.stdout.slice(0, 4)).toEqual([
      'Name: formatter',
      'Version: 1.2.0',
      'Description: Formats code',
      'Source: acme',
    ]);
  });

  test('a plugin that is not loaded prints the message and exits 1', () => {
    expect(planPluginDetails('formatter', undefined)).toEqual({
      stdout: [],
      stderr: [
        'Plugin "formatter" not found. Run `claude plugin list` to see installed plugins, or pass --plugin-dir <path> to load one from disk.',
      ],
      exitCode: 1,
    });
  });

  test('an @builtin id that is not found has its own message', () => {
    expect(planPluginDetails('tour@builtin', undefined).stderr).toEqual([
      'Plugin "tour@builtin" not found. Built-in plugins don\'t appear in `claude plugin list`. To see the ones available in a session, run /plugin in Claude Code and open the Installed tab.',
    ]);
  });
});

spec('cli.plugin.plugin details.Component inventory', () => {
  test('lists skills, agents, hooks, MCP servers and LSP servers, each with its count and names', () => {
    const out = planPluginDetails('formatter', {
      ...details,
      inventory: { ...details.inventory, skills: ['format', 'lint'] },
    }).stdout;
    const at = out.indexOf('Component inventory');
    expect(out.slice(at + 1, at + 6)).toEqual([
      '  Skills (2)  format, lint',
      '  Agents (0)',
      '  Hooks (1)  PostToolUse',
      '  MCP servers (1)  docs',
      '  LSP servers (0)',
    ]);
  });
});

spec('cli.plugin.plugin details.Projected token cost', () => {
  test('shows the always-on tokens, rounded', () => {
    const out = planPluginDetails('formatter', details).stdout;
    expect(out[out.indexOf('Projected token cost') + 1]).toContain('~1230 tokens');
  });
});

spec('cli.plugin.plugin details.Per-component (rounded)', () => {
  test('gives always-on and on-invoke estimates for each component', () => {
    const out = planPluginDetails('formatter', details).stdout;
    expect(out[out.indexOf('Per-component (rounded)') + 1]).toBe(
      '  skill format: always-on ~87, on-invoke ~2310',
    );
  });

  test('it is omitted when the plugin has none', () => {
    const out = planPluginDetails('formatter', { ...details, components: [] }).stdout;
    expect(out).not.toContain('Per-component (rounded)');
  });

  test('rounding', () => {
    expect([roundTokens(87.4), roundTokens(1234), roundTokens(99.6)]).toEqual([87, 1230, 100]);
  });
});
