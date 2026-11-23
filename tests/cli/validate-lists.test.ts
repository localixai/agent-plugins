/**
 * Spec §17.13.4 (plugins/cli-reference.md, "Output and exit codes": `provides`, `hosts`) and
 * Judgment J17.36: a plugin run lists the plugin's components by kind and the hosts it can reach,
 * after the findings; neither is a finding. A run that cannot list them gives `null`.
 */
import { describe, expect, test } from 'bun:test';
import { loadPlugin } from '../../src/index.ts';
import {
  pluginHosts,
  pluginProvides,
  validationListLines,
  validationLists,
  VALIDATION_PROVIDES_KINDS,
} from '../../src/cli/validate-lists.ts';
import { PROVIDES_KINDS, validateMod } from '../../src/mods/load/index.ts';
import { validationJson } from '../../src/cli/validate-run.ts';
import { json } from '../support/plugin-folder.ts';
import { memoryFiles } from '../support/ports.ts';
import { spec } from '../support/spec.ts';

const SKILL = '---\ndescription: d\n---\nbody';

async function listsOf(tree: Record<string, string>) {
  const { plugin } = await loadPlugin({ files: memoryFiles(tree) });
  if (plugin === undefined) throw new Error('the plugin does not load');
  const provides = pluginProvides(plugin);
  return { provides, hosts: pluginHosts(plugin, provides) };
}

const skillWithHosts = () =>
  listsOf({
    '.claude-plugin/plugin.json': json({ name: 't', network: { hosts: ['api.example.com'] } }),
    'skills/s/SKILL.md': SKILL,
  });

const lspOnly = () =>
  listsOf({
    '.claude-plugin/plugin.json': json({
      name: 'g',
      lspServers: { go: { command: 'gopls', extensionToLanguage: { '.go': 'go' } } },
    }),
  });

spec('cli.plugin.Output and exit codes.provides', () => {
  test('the components by kind, each named as a session names it', async () => {
    const { provides } = await skillWithHosts();
    expect(provides.skills).toEqual(['t:s']);
    expect(Object.keys(provides)).toEqual([
      'modules',
      'hooks',
      'mcpServers',
      'lspServers',
      'monitors',
      'skills',
      'commands',
      'agents',
      'outputStyles',
      'themes',
      'workflows',
      'bin',
    ]);
    expect((await lspOnly()).provides.lspServers).toEqual(['go']);
  });

  test('a run that cannot list them, such as a marketplace run, gives null', () => {
    const report = validationJson({ target: 'marketplace', path: '/m', issues: [], strict: false });
    expect(report.provides).toBeNull();
    expect(report.hosts).toBeNull();
  });

  test('the --json report carries what the plugin run listed', async () => {
    const lists = await skillWithHosts();
    const report = validationJson({
      target: 'plugin',
      path: '/t',
      issues: [],
      strict: false,
      lists,
    });
    expect(report.provides?.skills).toEqual(['t:s']);
    expect(report.success).toBe(true);
  });
});

spec('cli.plugin.Output and exit codes.hosts', () => {
  test('named hosts with their sources, unread parts with a reason, and what declares hosts', async () => {
    expect((await skillWithHosts()).hosts).toEqual({
      named: [{ host: 'api.example.com', sources: [{ type: 'declared' }] }],
      unread: [{ kind: 'skills', name: 't:s', reason: 'process' }],
      declared: { plugin: true, servers: [] },
    });
    expect((await lspOnly()).hosts).toEqual({
      named: [],
      unread: [{ kind: 'lspServers', name: 'go', reason: 'process' }],
      declared: { plugin: false, servers: [] },
    });
  });
});

describe('judgment J17.36: the text lines', () => {
  test('Provides, then the Hosts lines that apply', async () => {
    const one = await skillWithHosts();
    expect(validationListLines(one.provides, one.hosts)).toEqual([
      'Provides:',
      '  ❯ skills: t:s',
      'Hosts: api.example.com (declared)',
      'Hosts not read for: skills (1)',
    ]);
    const lsp = await lspOnly();
    expect(validationListLines(lsp.provides, lsp.hosts)).toEqual([
      'Provides:',
      '  ❯ lspServers: go',
      'Hosts not read for: lspServers (1)',
      'Hosts not declared: list what these parts reach under "network.hosts" in plugin.json',
    ]);
  });
});

describe('the hooks module in the lists (§10.4.3: `Provides:` lists `modules: hooks/register.js`, `Hosts:` the hosts its calls spell)', () => {
  const tree = {
    '.claude-plugin/plugin.json': json({ name: 'm', network: { hosts: ['api.example.com'] } }),
    'hooks/hooks.json': json({ modules: ['./register.js'] }),
    'hooks/register.js': `export function register(on) {
  on('session.start', async ($, e, next) => {
    await $.http.fetch('https://example.com/a')
    await $.http.fetch('https://api.example.com/b')
    return next(e)
  })
}`,
  };

  spec(
    ['cli.plugin.Output and exit codes.provides', 'cli.plugin.Output and exit codes.hosts'],
    () => {
      test("provides.modules and the named hosts come from the mods validator's results", async () => {
        const files = memoryFiles(tree);
        const { plugin } = await loadPlugin({ files });
        if (plugin === undefined) throw new Error('the plugin does not load');
        const mods = await validateMod({ files, target: '.' });
        const { provides, hosts } = validationLists(plugin, mods.contents);
        expect(provides.modules).toEqual(['hooks/register.js']);
        expect(hosts.named).toEqual([
          { host: 'api.example.com', sources: [{ type: 'declared' }, { type: 'module' }] },
          { host: 'example.com', sources: [{ type: 'module' }] },
        ]);
        expect(validationListLines(provides, hosts)).toEqual([
          'Provides:',
          '  ❯ modules: hooks/register.js',
          'Hosts: api.example.com (declared), example.com',
        ]);
      });
    },
  );

  test('judgment: the kinds the validator lists are the kinds of e.provides, in 2.1.330 order', () => {
    expect(VALIDATION_PROVIDES_KINDS.toSorted()).toEqual([...PROVIDES_KINDS]);
  });
});
