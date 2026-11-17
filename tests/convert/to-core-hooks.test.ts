/**
 * A Claude Code plugin's hooks and other components in a core package (Agent Plugins Extended
 * §18.4.3 item 4, the Claude Code row of §18.4.5): only `command` hooks on the five shared events,
 * through the adapter (J18.17), limited to the profile the matcher selects; every hook, field and
 * component the core lacks reported with its place. Outside `spec(...)`: chapter 18 has no
 * requirement ids.
 */
import { describe, expect, test } from 'bun:test';
import { claudeCodeToCore, type Conversion } from '../../src/entry/core.ts';
import { loadPlugin } from '../../src/index.ts';
import { memoryFiles } from '../support/ports.ts';
import { convertTree, coreHandlersOf } from '../support/release-guard.ts';

const json = (value: unknown) => JSON.stringify(value);
const MANIFEST = '.claude-plugin/plugin.json';

const tree = (manifest: Record<string, unknown> = {}, more: Record<string, string> = {}) => ({
  [MANIFEST]: json({ name: 'acme-tools', ...manifest }),
  ...more,
});
const notes = (converted: Conversion) =>
  converted.notes.map(({ outcome, path, field }) => ({ outcome, path, ...(field && { field }) }));
const hooksTree = (hooks: Record<string, unknown>) =>
  tree({}, { 'hooks/hooks.json': json({ hooks }) });
const hook = (more: Record<string, unknown> = {}) => ({
  type: 'command',
  command: 'node',
  args: ['${CLAUDE_PLUGIN_ROOT}/h.mjs'],
  ...more,
});
const flags = (args: readonly string[]) => args.slice(1, args.indexOf('--'));

describe('§18.4.3 item 4: hooks through the adapter', () => {
  test('a profile matcher limits the adapter to that profile; matching everything runs on every call', async () => {
    const converted = await convertTree(
      hooksTree({
        PreToolUse: [
          { matcher: 'Edit|Write|NotebookEdit', hooks: [hook()] },
          { matcher: 'mcp__.*', hooks: [hook()] },
          { matcher: '*', hooks: [hook({ timeout: 5 })] },
        ],
      }),
    );
    expect(coreHandlersOf(converted).PreToolUse?.map((one) => flags(one.args))).toEqual([
      ['--event', 'PreToolUse', '--profile', 'file.modify', '--exec'],
      ['--event', 'PreToolUse', '--profile', 'mcp', '--exec'],
      ['--event', 'PreToolUse', '--timeout', '5', '--exec'],
    ]);
    expect(converted.runtime).toBe('node');
  });

  test('shell form keeps its command and shell', async () => {
    const converted = await convertTree(
      hooksTree({ Stop: [{ hooks: [{ type: 'command', command: 'echo done', shell: 'bash' }] }] }),
    );
    expect(coreHandlersOf(converted).Stop?.[0]?.args.slice(1)).toEqual([
      '--event',
      'Stop',
      '--shell',
      'bash',
      '--',
      'echo done',
    ]);
  });

  test('other events, handler types, matchers and fields are not converted, each with its place', async () => {
    const converted = await convertTree(
      hooksTree({
        Notification: [{ hooks: [hook()] }],
        PreToolUse: [
          { matcher: 'Bash', hooks: [hook()] },
          { matcher: 'Glob', hooks: [hook()] },
          {
            hooks: [
              hook({ if: 'Bash(git *)' }),
              hook({ async: true }),
              hook({ once: true }),
              hook({ onFailure: 'block' }),
            ],
          },
          {
            hooks: [
              { type: 'http', url: 'https://example.com' },
              { type: 'prompt', prompt: 'ok?' },
            ],
          },
          { hooks: [hook({ args: ['${user_config.key}'] }), hook({ args: ['${PLUGIN_ROOT}/x'] })] },
        ],
        SessionStart: [{ matcher: 'resume', hooks: [hook()] }],
      }),
    );
    expect(converted.files.map((file) => file.path)).toEqual(['plugin.json']);
    expect(notes(converted).map((one) => one.field)).toEqual([
      'hooks.Notification[0].hooks[0]',
      'hooks.PreToolUse[0].hooks[0]',
      'hooks.PreToolUse[1].hooks[0]',
      'hooks.PreToolUse[2].hooks[0]',
      'hooks.PreToolUse[2].hooks[1]',
      'hooks.PreToolUse[2].hooks[2]',
      'hooks.PreToolUse[2].hooks[3]',
      'hooks.PreToolUse[3].hooks[0]',
      'hooks.PreToolUse[3].hooks[1]',
      'hooks.PreToolUse[4].hooks[0]',
      'hooks.PreToolUse[4].hooks[1]',
      'hooks.SessionStart[0].hooks[0]',
    ]);
    expect(converted.notes.every((one) => one.outcome === 'not-converted')).toBe(true);
  });

  test('behaviour that narrows or widens is reported: statusMessage, SessionStart for all, PostToolUse on failures', async () => {
    const converted = await convertTree(
      hooksTree({
        SessionStart: [{ hooks: [hook({ statusMessage: 'Loading' })] }],
        PostToolUse: [{ matcher: 'Read', hooks: [hook()] }],
      }),
    );
    expect(notes(converted)).toEqual([
      {
        outcome: 'reported',
        path: 'hooks/hooks.json',
        field: 'hooks.SessionStart[0].hooks[0].statusMessage',
      },
      { outcome: 'reported', path: 'hooks/hooks.json', field: 'hooks.SessionStart[0].hooks[0]' },
      { outcome: 'reported', path: 'hooks/hooks.json', field: 'hooks.PostToolUse[0].hooks[0]' },
    ]);
  });

  test('without an adapter every hook is reported and no hooks.json is written', async () => {
    const loaded = await loadPlugin({
      files: memoryFiles(hooksTree({ Stop: [{ hooks: [hook()] }] })),
    });
    if (loaded.plugin === undefined) throw new Error('the plugin does not load');
    const converted = claudeCodeToCore(loaded.plugin, { hookAdapter: false });
    expect(converted.files.map((file) => file.path)).toEqual(['plugin.json']);
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'hooks/hooks.json', field: 'hooks.Stop[0].hooks[0]' },
    ]);
  });
});

describe('§18.4.5: components without a core counterpart', () => {
  test('commands, agents, output styles, LSP servers, monitors, bin/, settings and mods are each reported', async () => {
    const converted = await convertTree(
      tree(
        {},
        {
          'commands/ship.md': '---\ndescription: Ship.\n---\nShip it.\n',
          'agents/reviewer.md': '---\nname: reviewer\ndescription: Reviews.\n---\nReview.\n',
          'output-styles/terse.md': '---\nname: terse\ndescription: Terse.\n---\nBe terse.\n',
          '.lsp.json': json({ go: { command: 'gopls', extensionToLanguage: { '.go': 'go' } } }),
          'monitors/monitors.json': json([
            { name: 'watch', command: 'tail -f log', description: 'Watch.' },
          ]),
          'bin/tool': '#!/bin/sh\n',
          'settings.json': json({ agent: 'reviewer' }),
          'hooks/hooks.json': json({ modules: ['mod.ts'] }),
        },
      ),
    );
    expect(notes(converted)).toEqual([
      { outcome: 'not-converted', path: 'commands/ship.md' },
      { outcome: 'not-converted', path: 'agents/reviewer.md' },
      { outcome: 'not-converted', path: 'output-styles/terse.md' },
      { outcome: 'not-converted', path: '.lsp.json', field: 'go' },
      { outcome: 'not-converted', path: 'monitors/monitors.json', field: 'watch' },
      { outcome: 'not-converted', path: 'bin/' },
      { outcome: 'not-converted', path: 'settings.json' },
      { outcome: 'not-converted', path: 'hooks/hooks.json', field: 'modules' },
    ]);
  });
});
