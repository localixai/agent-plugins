/**
 * The worked example of Agent Plugins Extended §18.3.5 (`release-guard`) through both converters
 * (§18.4.2, §18.4.3): each half becomes the other with the same manifest, the same servers (plus the
 * environment bridge of J18.18) and the same denial through a hook adapter (§18.4.4, J18.17); a core
 * package converted to Claude Code and back is the package it was, and a second round trip changes
 * nothing (§18.4.1 item 5). Chapter 18 has no requirement ids, so these tests are outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { validatePlugin } from '../../src/index.ts';
import { loadCorePackage } from '../../src/entry/core.ts';
import { hookArgs, NODE, runNode, writeTree } from '../support/adapter-run.ts';
import { memoryFiles } from '../support/ports.ts';
import {
  CLAUDE_HALF,
  CLAUDE_HOOKS,
  CLAUDE_MANIFEST,
  CLAUDE_MCP,
  convertTree,
  CORE_HALF,
  CORE_HOOKS,
  CORE_MANIFEST,
  CORE_MCP,
  DENIAL,
  outputOf,
  SHARED_FILES,
  written,
} from '../support/release-guard.ts';

const CLAUDE_BRIDGE = {
  CLAUDE_PLUGIN_ROOT: '${PLUGIN_ROOT}',
  CLAUDE_PLUGIN_DATA: '${PLUGIN_DATA}',
};
const CORE_BRIDGE = { PLUGIN_ROOT: '${CLAUDE_PLUGIN_ROOT}', PLUGIN_DATA: '${CLAUDE_PLUGIN_DATA}' };

const withEnv = (mcp: typeof CORE_MCP | typeof CLAUDE_MCP, env: Record<string, string>) => ({
  ...mcp,
  mcpServers: { notes: { ...mcp.mcpServers.notes, env } },
});

const without = (tree: Readonly<Record<string, string>>, paths: readonly string[]) =>
  Object.fromEntries(Object.entries(tree).filter(([path]) => !paths.includes(path)));

describe('§18.3.5 core half to Claude Code (§18.4.2)', () => {
  test('the manifest is the Claude Code manifest of the example, nothing reported', async () => {
    const converted = await convertTree(CORE_HALF);
    expect(converted.stopped).toBe(false);
    expect(converted.notes).toEqual([]);
    expect(outputOf(converted, '.claude-plugin/plugin.json')).toEqual(CLAUDE_MANIFEST);
    expect(outputOf(converted, '.mcp.json')).toEqual(withEnv(CLAUDE_MCP, CORE_BRIDGE));
    expect(converted.runtime).toBe('node');
  });

  test('the output passes claude plugin validate and keeps the core half', async () => {
    const converted = await convertTree(CORE_HALF);
    const tree = written(CORE_HALF, converted);
    expect((await validatePlugin({ files: memoryFiles(tree) })).issues).toEqual([]);
    expect(tree['plugin.json']).toBe(CORE_HALF['plugin.json']);
  });

  test.skipIf(NODE === null)('the adapter denies npm publish in Bash with the reason', async () => {
    const converted = await convertTree(CORE_HALF);
    const root = writeTree(written(CORE_HALF, converted));
    const args = hookArgs(outputOf(converted, 'hooks/hooks.json'), 'PreToolUse').map((arg) =>
      arg.replaceAll('${CLAUDE_PLUGIN_ROOT}', root),
    );
    const env = { CLAUDE_PLUGIN_ROOT: root, CLAUDE_PLUGIN_DATA: join(root, '.data') };
    const call = (command: string) =>
      runNode(
        args,
        {
          hook_event_name: 'PreToolUse',
          session_id: 's',
          cwd: root,
          tool_name: 'Bash',
          tool_input: { command },
          tool_use_id: 'toolu_1',
        },
        { cwd: root, env },
      );
    expect(JSON.parse(call('npm publish --access public').stdout)).toEqual({
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        permissionDecision: 'deny',
        permissionDecisionReason: DENIAL,
      },
    });
    expect(call('npm test')).toEqual({ status: 0, stdout: '', stderr: '' });
  });
});

describe('§18.3.5 Claude Code half to the core (§18.4.3)', () => {
  test('the manifest is the core manifest of the example, nothing reported', async () => {
    const converted = await convertTree(CLAUDE_HALF);
    expect(converted.notes).toEqual([]);
    expect(outputOf(converted, 'plugin.json')).toEqual(CORE_MANIFEST);
    expect(outputOf(converted, 'mcp.json')).toEqual(withEnv(CORE_MCP, CLAUDE_BRIDGE));
  });

  test('the output loads as a core package with every server and handler', async () => {
    const converted = await convertTree(CLAUDE_HALF);
    const tree = without(written(CLAUDE_HALF, converted), ['.claude-plugin/plugin.json']);
    const loaded = await loadCorePackage({ files: memoryFiles(tree) });
    expect(loaded.issues).toEqual([]);
    expect(Object.keys(loaded.package?.mcpServers ?? {})).toEqual(['notes']);
    expect(loaded.package?.hooks.PreToolUse).toHaveLength(1);
    expect(loaded.package?.skills.map((skill) => skill.name)).toEqual(['release-notes']);
  });

  test.skipIf(NODE === null)('the adapter denies a shell call with the reason', async () => {
    const converted = await convertTree(CLAUDE_HALF);
    const root = writeTree(written(CLAUDE_HALF, converted));
    const args = hookArgs(outputOf(converted, 'hooks.json'), 'PreToolUse').map((arg) =>
      arg.replaceAll('${PLUGIN_ROOT}', root),
    );
    const env = { PLUGIN_ROOT: root, PLUGIN_DATA: join(root, '.data') };
    const call = (profile: string, command: string) =>
      runNode(
        args,
        {
          hook_event_name: 'PreToolUse',
          session_id: 's',
          cwd: '/work',
          tool_use_id: 'call_1',
          tool_name: 'shell',
          tool_input: { command },
          tool_profile: { name: profile, input: { command } },
        },
        { cwd: root, env },
      );
    expect(JSON.parse(call('shell', 'npm publish --access public').stdout)).toEqual({
      decision: 'deny',
      reason: DENIAL,
    });
    expect(call('shell', 'npm test').stdout).toBe('');
    expect(call('file.read', 'npm publish').stdout).toBe('');
  });
});

describe('round trips (§18.4.1 item 5)', () => {
  const CORE_FILES = ['plugin.json', 'mcp.json', 'hooks.json'];
  const CLAUDE_FILES = ['.claude-plugin/plugin.json', '.mcp.json', 'hooks/hooks.json'];

  test('core to Claude Code and back: the same package, with the bridge of J18.18', async () => {
    const there = await convertTree(CORE_HALF);
    const back = await convertTree(without(written(CORE_HALF, there), CORE_FILES));
    expect(back.notes).toEqual([]);
    expect(outputOf(back, 'plugin.json')).toEqual(CORE_MANIFEST);
    expect(outputOf(back, 'hooks.json')).toEqual(CORE_HOOKS);
    expect(outputOf(back, 'mcp.json')).toEqual(withEnv(CORE_MCP, CLAUDE_BRIDGE));
  });

  test('Claude Code to core and back: the same plugin, with the bridge of J18.18', async () => {
    const there = await convertTree(CLAUDE_HALF);
    const back = await convertTree(without(written(CLAUDE_HALF, there), CLAUDE_FILES));
    expect(back.notes).toEqual([]);
    expect(outputOf(back, '.claude-plugin/plugin.json')).toEqual(CLAUDE_MANIFEST);
    expect(outputOf(back, 'hooks/hooks.json')).toEqual(CLAUDE_HOOKS);
    expect(outputOf(back, '.mcp.json')).toEqual(withEnv(CLAUDE_MCP, CORE_BRIDGE));
  });

  test('a second round trip writes the same bytes', async () => {
    const once = await convertTree(CORE_HALF);
    const core = await convertTree(without(written(CORE_HALF, once), CORE_FILES));
    const coreTree = { ...SHARED_FILES, ...written({}, core) };
    const twice = await convertTree(without(coreTree, ['hooks/run-claude-code-handler.mjs']));
    expect(twice.files).toEqual(once.files);
    expect(await convertTree(CORE_HALF)).toEqual(once);
  });
});
