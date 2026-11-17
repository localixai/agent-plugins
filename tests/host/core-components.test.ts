/**
 * A core package's skills, MCP servers and plugin identity in a session (Agent Plugins Extended
 * §18.1.1 item 3, §18.1.3, §18.1.4, §18.1.5; Judgments J18.10–J18.15; core §7.1, §7.2.1, §9.1, §9.2).
 * Chapter 18 carries no requirement ids, so these are judgment-style tests outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { createPolicyView } from '../../src/settings/index.ts';
import { CLAUDE_ID, CORE_ID, coreFiles, corePackage, coreHostOf } from '../support/core-host.ts';
import { SESSION, signal } from '../support/host.ts';
import { layerOf } from '../support/settings-layers.ts';

describe('§18.1.5 items 1, 3 and 6: a core package is a plugin by its id', () => {
  test('the plugin list names it and says it is a core package', async () => {
    const { host } = await coreHostOf();
    expect(host.plugins).toEqual([
      { id: CLAUDE_ID, name: 'guard', enabled: true },
      { id: CORE_ID, name: 'acme.tools', enabled: true, core: true },
    ]);
  });

  test('enabledPlugins turns it off by id', async () => {
    const settings = createPolicyView([layerOf('user', { enabledPlugins: { [CORE_ID]: false } })]);
    const { host } = await coreHostOf({ settings });
    expect(host.plugins.find((one) => one.id === CORE_ID)?.enabled).toBe(false);
    expect(
      host
        .openSession(SESSION)
        .skills()
        .menu.map((one) => one.name),
    ).toEqual([]);
  });

  test('a session brings its own core package beside the host plugins', async () => {
    const { host } = await coreHostOf({ claude: false });
    const own = await corePackage({
      ...coreFiles(),
      'plugin.json': JSON.stringify({
        $schema: 'https://agent-plugins.org/schemas/1.1.0/plugin.schema.json',
        name: 'local-pkg',
      }),
    });
    const session = host.openSession({
      ...SESSION,
      corePackages: [{ id: 'local-pkg@inline', package: own }],
    });
    expect(session.plugins.map((one) => one.id)).toEqual([CORE_ID, 'local-pkg@inline']);
  });
});

describe('§18.1.5 item 5, J18.14: its skills are plugin skills without expansion', () => {
  test('/acme.tools:release is listed and invocable; a Claude Code field has no effect', async () => {
    const { host } = await coreHostOf();
    const listing = host.openSession(SESSION).skills();
    expect(listing.menu.map((one) => one.name)).toEqual(['acme.tools:release']);
    // `disable-model-invocation: true` is not an Agent Skills field: the model is told of it.
    expect(listing.model.entries.map((one) => one.name)).toEqual(['acme.tools:release']);
  });

  test('the body is used as written and the arguments are appended', async () => {
    const { host, runner } = await coreHostOf();
    const expansion = await host
      .openSession(SESSION)
      .expandSkill('acme.tools:release', { arguments: '1.2.0', signal: signal() });
    if (!expansion.ok) throw new Error(JSON.stringify(expansion.issues));
    expect(expansion.text).toContain(
      'Release $ARGUMENTS from ${CLAUDE_PLUGIN_ROOT} with !`git tag`.\n\nARGUMENTS: 1.2.0',
    );
    expect(expansion.grants).toEqual(['Bash(git:*)']);
    expect(runner.calls).toHaveLength(0);
  });
});

describe('§18.1.4 and §18.1.3, J18.10–J18.12: its MCP servers', () => {
  test('servers register as plugin:<name>:<server>, expanded by core §9.2 only', async () => {
    const { host } = await coreHostOf();
    const { resolved, issues } = await host.openSession(SESSION).mcpServers();
    expect(issues).toEqual([]);
    expect(resolved.map((one) => [one.key, one.pluginId])).toEqual([
      ['plugin:acme.tools:notes', CORE_ID],
      ['plugin:acme.tools:remote', CORE_ID],
    ]);
    const [notes, remote] = resolved;
    const root = `/plugins/${CORE_ID}`;
    expect(notes?.server).toEqual({
      type: 'stdio',
      command: `${root}/bin/notes`,
      args: ['--data', `/data/${CORE_ID}/notes`, '${CLAUDE_PLUGIN_ROOT}'],
      env: { NOTES_HOME: `${root}/share` },
    });
    expect(notes?.environment).toEqual({
      PLUGIN_ROOT: root,
      PLUGIN_DATA: `/data/${CORE_ID}`,
      CLAUDE_PROJECT_DIR: '/work/app',
    });
    expect(notes?.core).toEqual({ cwd: root });
    expect(notes?.start).toEqual({
      form: 'exec',
      command: `${root}/bin/notes`,
      args: ['--data', `/data/${CORE_ID}/notes`, '${CLAUDE_PLUGIN_ROOT}'],
    });
    expect(remote?.server).toEqual({ type: 'http', url: 'https://notes.example/mcp' });
  });

  test('§18.1.1 item 3: a .mcp.json or hooks/hooks.json in a core package is an ordinary file', async () => {
    const files = {
      ...coreFiles({}),
      '.mcp.json': JSON.stringify({ mcpServers: { cc: { command: 'never' } } }),
      'hooks/hooks.json': JSON.stringify({
        hooks: { Stop: [{ hooks: [{ type: 'command', command: 'never' }] }] },
      }),
    };
    const { host, runner } = await coreHostOf({ claude: false, core: await corePackage(files) });
    const session = host.openSession(SESSION);
    const keys = (await session.mcpServers()).resolved.map((one) => one.key);
    expect(keys).not.toContain('plugin:acme.tools:cc');
    await session.dispatch('Stop', { stop_hook_active: false }, { signal: signal() });
    expect(runner.calls).toHaveLength(0);
  });
});
