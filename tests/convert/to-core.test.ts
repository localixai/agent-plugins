/**
 * A Claude Code plugin to a core package (Agent Plugins Extended §18.4.3, the rules of §18.4.1 and
 * the Claude Code row of §18.4.5): a name core §5.5 refuses stops; the nine shared fields are written
 * and every other manifest field reported; skills convert only from `skills/<name>/`; one `mcp.json`
 * with the transports, commands and placeholders of item 3; hooks only as `command` hooks on the five
 * events through the adapter, limited to the profile the matcher selects; every component the core
 * lacks reported. Outside `spec(...)`: chapter 18 has no requirement ids.
 */
import { describe, expect, test } from 'bun:test';
import type { Conversion } from '../../src/entry/core.ts';
import { convertTree, outputOf, serversOf } from '../support/release-guard.ts';

const json = (value: unknown) => JSON.stringify(value);
const MANIFEST = '.claude-plugin/plugin.json';

const tree = (manifest: Record<string, unknown> = {}, more: Record<string, string> = {}) => ({
  [MANIFEST]: json({ name: 'acme-tools', ...manifest }),
  ...more,
});
const notes = (converted: Conversion) =>
  converted.notes.map(({ outcome, path, field }) => ({ outcome, path, ...(field && { field }) }));
const mcpTree = (servers: Record<string, unknown>) =>
  tree({}, { '.mcp.json': json({ mcpServers: servers }) });
const skill = (frontmatter: string, body = 'Do it.') => `---\n${frontmatter}\n---\n${body}\n`;

describe('§18.4.3 item 1: the manifest', () => {
  test('the canonical $schema and the shared fields; every other field reported', async () => {
    const converted = await convertTree(
      tree({
        version: '1.0.0',
        license: 'MIT',
        userConfig: {},
        defaultEnabled: false,
        icon: 'icon.png',
      }),
    );
    expect(outputOf(converted, 'plugin.json')).toEqual({
      $schema: 'https://agent-plugins.org/schemas/1.1.0/plugin.schema.json',
      name: 'acme-tools',
      version: '1.0.0',
      license: 'MIT',
    });
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: MANIFEST, field: 'icon' },
      { outcome: 'not-converted', path: MANIFEST, field: 'defaultEnabled' },
      { outcome: 'not-converted', path: MANIFEST, field: 'userConfig' },
    ]);
  });

  test('§18.4.1 item 2: a name core §5.5 refuses stops the conversion; it is never renamed', async () => {
    const converted = await convertTree(tree({ name: 'Acme_Tools' }));
    expect(converted.stopped).toBe(true);
    expect(converted.files).toEqual([]);
    expect(notes(converted)[0]).toEqual({ outcome: 'stopped', path: MANIFEST, field: 'name' });
  });
});

describe('§18.4.3 item 2: skills', () => {
  test('a skill under skills/<name>/ with the six fields converts with no note', async () => {
    const converted = await convertTree(
      tree({}, { 'skills/deploy/SKILL.md': skill('name: deploy\ndescription: Deploys.') }),
    );
    expect(converted.notes).toEqual([]);
  });

  test('Claude Code fields and §4.8 body features are reported; a skill the core skips is not converted', async () => {
    const converted = await convertTree(
      tree(
        {},
        {
          'skills/deploy/SKILL.md': skill(
            'name: deploy\ndescription: Deploys.\nmodel: haiku',
            'Deploy $ARGUMENTS.',
          ),
          'skills/Bad_Name/SKILL.md': skill('description: Bad.'),
        },
      ),
    );
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'skills/Bad_Name/SKILL.md' },
      { outcome: 'reported', path: 'skills/deploy/SKILL.md', field: 'model' },
      { outcome: 'reported', path: 'skills/deploy/SKILL.md' },
    ]);
  });

  test('a skill in another folder (a manifest skills path) is not converted', async () => {
    const converted = await convertTree(
      tree(
        { skills: ['./extra/'] },
        { 'extra/review/SKILL.md': skill('name: review\ndescription: Reviews.') },
      ),
    );
    expect(notes(converted)).toEqual([{ outcome: 'not-converted', path: 'extra/review/SKILL.md' }]);
  });
});

describe('§18.4.3 item 3: one mcp.json', () => {
  test('transports, ${CLAUDE_PLUGIN_ROOT}/ commands, renamed placeholders and the CLAUDE_* bridge (J18.18)', async () => {
    const converted = await convertTree({
      'server/run': '#!/bin/sh\n',
      ...mcpTree({
        local: { command: '${CLAUDE_PLUGIN_ROOT}/server/run', args: ['${CLAUDE_PLUGIN_DATA}/db'] },
        web: { type: 'http', url: 'https://example.com/mcp', headers: { 'X-Key': 'k' } },
        events: { type: 'sse', url: 'https://example.com/sse' },
      }),
    });
    expect(serversOf(converted, 'mcp.json')).toEqual({
      local: {
        type: 'stdio',
        command: './server/run',
        args: ['${PLUGIN_DATA}/db'],
        env: { CLAUDE_PLUGIN_ROOT: '${PLUGIN_ROOT}', CLAUDE_PLUGIN_DATA: '${PLUGIN_DATA}' },
      },
      web: { type: 'streamable-http', url: 'https://example.com/mcp', headers: { 'X-Key': 'k' } },
      events: { type: 'sse', url: 'https://example.com/sse' },
    });
    expect(converted.notes).toEqual([]);
  });

  test('the manifest inline servers merge in, the later name winning (§2.9)', async () => {
    const converted = await convertTree({
      ...mcpTree({ db: { command: 'old' } }),
      [MANIFEST]: json({
        name: 'acme-tools',
        mcpServers: { db: { command: 'new' }, other: { command: 'x' } },
      }),
    });
    expect(Object.keys(serversOf(converted, 'mcp.json')).toSorted()).toEqual(['db', 'other']);
  });

  test('what the core cannot carry is not converted, each with its server', async () => {
    const converted = await convertTree(
      mcpTree({
        ws: { type: 'ws', url: 'wss://example.com' },
        helper: { type: 'http', url: 'https://example.com', headersHelper: 'bin/h' },
        timed: { command: 'node', timeout: 5000 },
        variable: { command: 'node', args: ['${HOME}/x'] },
        option: { command: 'node', env: { KEY: '${user_config.key}' } },
        script: { command: 'bin/server' },
        plain: { type: 'http', url: 'http://example.com/mcp' },
        header: { type: 'sse', url: 'https://example.com', headers: { A: '${TOKEN}' } },
        ok: { command: 'node' },
      }),
    );
    expect(Object.keys(serversOf(converted, 'mcp.json'))).toEqual(['ok']);
    expect(notes(converted).map((one) => one.field)).toEqual(
      ['ws', 'helper', 'timed', 'variable', 'option', 'script', 'plain', 'header'].map(
        (name) => `mcpServers.${name}`,
      ),
    );
  });
});
