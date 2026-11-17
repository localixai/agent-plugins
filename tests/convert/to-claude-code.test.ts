/**
 * A core package to a Claude Code plugin (Agent Plugins Extended §18.4.2, the rules of §18.4.1 and
 * the core row of §18.4.5): the manifest's nine fields, nothing invented and nothing renamed;
 * skills used as they are and reported when Claude Code would expand them; `.mcp.json` by the table
 * of §18.3.3 with the environment bridged (J18.18); hooks only through the adapter (J18.17); every
 * field, entry and directory left behind reported with its path. Outside `spec(...)`: chapter 18 has
 * no requirement ids.
 */
import { describe, expect, test } from 'bun:test';
import { coreToClaudeCode, loadCorePackage, type Conversion } from '../../src/entry/core.ts';
import { memoryFiles } from '../support/ports.ts';
import { claudeGroupsOf, convertTree, outputOf, serverNamesOf } from '../support/release-guard.ts';

const schema = (document: string) =>
  `https://agent-plugins.org/schemas/1.1.0/${document}.schema.json`;
const json = (value: unknown) => JSON.stringify(value);

const tree = (manifest: Record<string, unknown>, more: Record<string, string> = {}) => ({
  'plugin.json': json({ $schema: schema('plugin'), name: 'acme-tools', ...manifest }),
  ...more,
});

const mcpTree = (servers: Record<string, unknown>) =>
  tree({}, { 'mcp.json': json({ $schema: schema('mcp'), mcpServers: servers }) });
const hooksTree = (hooks: Record<string, unknown>) =>
  tree({}, { 'hooks.json': json({ $schema: schema('hooks'), hooks }) });

const notes = (converted: Conversion) =>
  converted.notes.map(({ outcome, path, field }) => ({ outcome, path, ...(field && { field }) }));

describe('§18.4.2 item 1: the manifest', () => {
  test('the nine shared fields are copied and $schema dropped', async () => {
    const manifest = {
      displayName: 'Acme',
      version: '2.0.0',
      description: 'Tools.',
      author: { name: 'Acme', email: 'a@example.com' },
      homepage: 'https://example.com',
      repository: 'https://github.com/acme/tools',
      license: 'MIT',
      keywords: ['a'],
    };
    const converted = await convertTree(tree(manifest));
    expect(outputOf(converted, '.claude-plugin/plugin.json')).toEqual({
      name: 'acme-tools',
      ...manifest,
    });
    expect(converted.files.map((file) => file.path)).toEqual(['.claude-plugin/plugin.json']);
  });

  test('an author without name and a homepage that is no URL are left out and reported, never invented', async () => {
    const converted = await convertTree(
      tree({ author: { email: 'a@example.com' }, homepage: 'example.com' }),
    );
    expect(outputOf(converted, '.claude-plugin/plugin.json')).toEqual({ name: 'acme-tools' });
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'plugin.json', field: 'author' },
      { outcome: 'not-converted', path: 'plugin.json', field: 'homepage' },
    ]);
  });

  test('extensions and extension directories are reported, whatever namespaces the host implements', async () => {
    const converted = await convertTree(
      tree(
        { extensions: { 'com.example.client': { on: true } } },
        { 'org.other.client/hooks/hooks.json': '{}' },
      ),
    );
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'plugin.json', field: 'extensions.com.example.client' },
      { outcome: 'not-converted', path: 'org.other.client/' },
    ]);
  });

  test('§18.4.1 item 2: a name Claude Code reports stays; one its validator refuses stops the conversion', async () => {
    const dotted = await convertTree({
      'plugin.json': json({ $schema: schema('plugin'), name: 'acme.tools' }),
    });
    expect(dotted.stopped).toBe(false);
    expect(outputOf(dotted, '.claude-plugin/plugin.json')).toEqual({ name: 'acme.tools' });
    expect(notes(dotted)).toEqual([{ outcome: 'reported', path: 'plugin.json', field: 'name' }]);
    const reserved = await convertTree({
      'plugin.json': json({ $schema: schema('plugin'), name: 'claude-code-tools' }),
    });
    expect(reserved.stopped).toBe(true);
    expect(reserved.files).toEqual([]);
    expect(notes(reserved)).toContainEqual({
      outcome: 'stopped',
      path: 'plugin.json',
      field: 'name',
    });
  });
});

const skill = (body: string) => ({
  'skills/notes/SKILL.md': `---\nname: notes\ndescription: Notes.\n---\n${body}\n`,
});

describe('§18.4.2 item 2: skills', () => {
  test('skills/ is used as it is: nothing written, nothing reported for a plain body', async () => {
    const converted = await convertTree(tree({}, skill('Write the notes.')));
    expect(converted.files.map((file) => file.path)).toEqual(['.claude-plugin/plugin.json']);
    expect(converted.notes).toEqual([]);
  });

  test('a body with text Claude Code expands or runs is reported; an escaped placeholder is not', async () => {
    const converted = await convertTree(
      tree(
        {},
        skill('Use $ARGUMENTS and $1 in ${CLAUDE_SKILL_DIR}; run !`git status`; costs \\$5.'),
      ),
    );
    expect(converted.notes).toHaveLength(1);
    expect(converted.notes[0]?.outcome).toBe('reported');
    expect(converted.notes[0]?.message).toContain(
      'argument placeholders $ARGUMENTS, $1; variables ${CLAUDE_SKILL_DIR}; injected commands (git status)',
    );
  });
});

describe('§18.4.2 item 3: MCP servers by the table of §18.3.3', () => {
  test('transports, a ./ command, renamed placeholders and the PLUGIN_* bridge (J18.18)', async () => {
    const converted = await convertTree({
      'server/run': '#!/bin/sh\n',
      ...mcpTree({
        local: {
          type: 'stdio',
          command: './server/run',
          args: ['${PLUGIN_DATA}/db'],
          env: { MODE: 'x' },
        },
        web: { type: 'streamable-http', url: 'https://example.com/mcp', headers: { 'X-Key': 'k' } },
        events: { type: 'sse', url: 'https://example.com/sse' },
      }),
    });
    expect(outputOf(converted, '.mcp.json')).toEqual({
      mcpServers: {
        local: {
          type: 'stdio',
          command: '${CLAUDE_PLUGIN_ROOT}/server/run',
          args: ['${CLAUDE_PLUGIN_DATA}/db'],
          env: {
            MODE: 'x',
            PLUGIN_ROOT: '${CLAUDE_PLUGIN_ROOT}',
            PLUGIN_DATA: '${CLAUDE_PLUGIN_DATA}',
          },
        },
        web: { type: 'http', url: 'https://example.com/mcp', headers: { 'X-Key': 'k' } },
        events: { type: 'sse', url: 'https://example.com/sse' },
      },
    });
    expect(converted.notes).toEqual([]);
  });

  test('cwd and extensions are left out and reported; the server is still written', async () => {
    const converted = await convertTree(
      mcpTree({
        db: {
          type: 'stdio',
          command: 'npx',
          cwd: '${PLUGIN_ROOT}/db',
          extensions: { 'com.example.client': {} },
        },
      }),
    );
    expect(serverNamesOf(converted, '.mcp.json')).toEqual(['db']);
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'mcp.json', field: 'mcpServers.db.cwd' },
      {
        outcome: 'not-converted',
        path: 'mcp.json',
        field: 'mcpServers.db.extensions.com.example.client',
      },
    ]);
  });

  test('a ${ left in args, env, url or headers: the server is not written', async () => {
    const converted = await convertTree(
      mcpTree({
        a: { type: 'stdio', command: 'node', args: ['${HOME}/x'] },
        b: { type: 'stdio', command: 'node', env: { TOKEN: '${TOKEN}' } },
        c: { type: 'sse', url: 'https://example.com/${x}' },
        d: { type: 'streamable-http', url: 'https://example.com', headers: { A: '${A}' } },
        e: { type: 'stdio', command: 'node' },
      }),
    );
    expect(serverNamesOf(converted, '.mcp.json')).toEqual(['e']);
    expect(notes(converted).map((one) => one.field)).toEqual([
      'mcpServers.a',
      'mcpServers.b',
      'mcpServers.c',
      'mcpServers.d',
    ]);
  });

  test('an entry the core skips is not written, and the skip is reported', async () => {
    const converted = await convertTree(
      mcpTree({
        bad: { type: 'stdio', command: 'two words' },
        ok: { type: 'stdio', command: 'node' },
      }),
    );
    expect(serverNamesOf(converted, '.mcp.json')).toEqual(['ok']);
    expect(notes(converted)).toEqual([{ outcome: 'not-converted', path: 'mcpServers.bad' }]);
  });
});

const handler = { command: 'node', args: ['${PLUGIN_ROOT}/h.mjs', '${PLUGIN_DATA}'] };
const adapter = (event: string) => ({
  type: 'command',
  command: 'node',
  args: [
    '${CLAUDE_PLUGIN_ROOT}/hooks/run-core-handler.mjs',
    event,
    '--',
    'node',
    '${CLAUDE_PLUGIN_ROOT}/h.mjs',
    '${CLAUDE_PLUGIN_DATA}',
  ],
});

describe('§18.4.2 item 4: hooks through the adapter', () => {
  test('registered as §18.3.4 item 3 says: SessionStart on new sessions, PostToolUse twice, no matcher otherwise', async () => {
    const events = ['SessionStart', 'UserPromptSubmit', 'PreToolUse', 'PostToolUse', 'Stop'];
    const converted = await convertTree(
      hooksTree(Object.fromEntries(events.map((event) => [event, [handler]]))),
    );
    expect(outputOf(converted, 'hooks/hooks.json')).toEqual({
      hooks: {
        SessionStart: [{ matcher: 'startup|clear|fork', hooks: [adapter('SessionStart')] }],
        UserPromptSubmit: [{ hooks: [adapter('UserPromptSubmit')] }],
        PreToolUse: [{ hooks: [adapter('PreToolUse')] }],
        PostToolUse: [{ hooks: [adapter('PostToolUse')] }],
        PostToolUseFailure: [{ hooks: [adapter('PostToolUse')] }],
        Stop: [{ hooks: [adapter('Stop')] }],
      },
    });
    expect(converted.files.map((file) => file.path)).toContain('hooks/run-core-handler.mjs');
    expect(converted.runtime).toBe('node');
  });

  test('a handler with another ${ in args is not converted; extensions are reported', async () => {
    const converted = await convertTree(
      hooksTree({
        Stop: [
          { command: 'node', args: ['${HOME}/x.mjs'] },
          { command: 'node', args: ['y.mjs'], extensions: { 'com.example.client': {} } },
        ],
      }),
    );
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'hooks.json', field: 'hooks.Stop[0]' },
      {
        outcome: 'not-converted',
        path: 'hooks.json',
        field: 'hooks.Stop[1].extensions.com.example.client',
      },
    ]);
    expect(claudeGroupsOf(converted).Stop).toHaveLength(1);
  });

  test('J18.23: a UserPromptSubmit handler is listed as also running on task notifications and automatic continuations', async () => {
    const converted = await convertTree(hooksTree({ UserPromptSubmit: [handler] }));
    expect(claudeGroupsOf(converted).UserPromptSubmit).toHaveLength(1);
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'hooks.json', field: 'hooks.UserPromptSubmit[0]' },
    ]);
    expect(converted.notes[0]?.message).toContain(
      'also runs on task notifications and automatic continuations',
    );
  });

  test('without an adapter every handler is reported and no hooks file is written', async () => {
    const loaded = await loadCorePackage({ files: memoryFiles(hooksTree({ Stop: [handler] })) });
    if (loaded.package === undefined) throw new Error('the package does not load');
    const converted = coreToClaudeCode(loaded.package, { hookAdapter: false });
    expect(converted.files.map((file) => file.path)).toEqual(['.claude-plugin/plugin.json']);
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'hooks.json', field: 'hooks.Stop[0]' },
    ]);
    expect(converted.runtime).toBeUndefined();
  });
});

describe('§18.4.1 item 5: deterministic', () => {
  test('the same package gives the same bytes', async () => {
    const input = mcpTree({
      b: { type: 'stdio', command: 'node' },
      a: { type: 'sse', url: 'https://x.example' },
    });
    expect(await convertTree(input)).toEqual(await convertTree(input));
  });
});
