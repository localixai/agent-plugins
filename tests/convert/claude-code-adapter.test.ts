/**
 * The hook adapter that runs a Claude Code `command` hook under a core client (Agent Plugins
 * Extended §18.4.4, J18.17), started with `node` as a core client starts a handler: the hook started
 * as Claude Code would (item 1), its input built from the core input with nothing invented (item 2),
 * exit 2 and plain stdout translated by meaning (item 3), one valid core result or nothing, always
 * exit 0 (item 4). Outside `spec(...)`: chapter 18 has no requirement ids.
 */
import { describe, expect, test } from 'bun:test';
import { realpathSync } from 'node:fs';
import { join } from 'node:path';
import { ANSWER, hookArgs, NODE, runNode, writeTree } from '../support/adapter-run.ts';
import { convertTree, outputOf, written } from '../support/release-guard.ts';

const json = (value: unknown) => JSON.stringify(value);

async function claudePlugin(event: string, hook: Record<string, unknown>, matcher?: string) {
  const tree = {
    '.claude-plugin/plugin.json': json({ name: 'probe' }),
    'hooks/hooks.json': json({
      hooks: { [event]: [{ ...(matcher && { matcher }), hooks: [{ type: 'command', ...hook }] }] },
    }),
    'handlers/answer.mjs': ANSWER,
  };
  const converted = await convertTree(tree);
  const root = realpathSync(writeTree(written(tree, converted)));
  const args = hookArgs(outputOf(converted, 'hooks.json'), event).map((arg) =>
    arg.replaceAll('${PLUGIN_ROOT}', root),
  );
  const env = { PLUGIN_ROOT: root, PLUGIN_DATA: join(root, 'data') };
  return (input: Record<string, unknown>) =>
    runNode(
      args,
      { hook_event_name: event, session_id: 's1', cwd: '/work', ...input },
      { cwd: root, env },
    );
}

const answering = (output: string, status = '0', stderr = '') => ({
  command: 'node',
  args: ['${CLAUDE_PLUGIN_ROOT}/handlers/answer.mjs', output, status, stderr],
});

describe.skipIf(NODE === null)('a Claude Code hook under a core client', () => {
  test('items 1, 2: CLAUDE_* set, CLAUDE_PROJECT_DIR is the input cwd, only fields the core supplies', async () => {
    const script =
      'printf \'{"hookSpecificOutput":{"additionalContext":"%s %s %s"}}\' "$CLAUDE_PROJECT_DIR" "$(cat | tr -d \'"\')" "$PWD"';
    const run = await claudePlugin('PostToolUse', { command: script });
    const out = JSON.parse(
      run({
        tool_use_id: 't',
        tool_name: 'Bash',
        tool_input: { command: 'ls' },
        tool_result: { stdout: 'x' },
      }).stdout,
    );
    expect(out.additional_context).toStartWith(
      '/work {hook_event_name:PostToolUse,session_id:s1,cwd:/work,tool_name:Bash,tool_input:{command:ls},tool_use_id:t,tool_response:{stdout:x}} /',
    );
  });

  test('item 3: exit 2 is the block of each event, with stderr as its reason', async () => {
    const block = answering('', '2', 'stop it');
    expect(
      JSON.parse((await claudePlugin('UserPromptSubmit', block))({ prompt: 'p' }).stdout),
    ).toEqual({
      decision: 'deny',
      reason: 'stop it',
    });
    expect(
      JSON.parse((await claudePlugin('Stop', block))({ continuation_active: false }).stdout),
    ).toEqual({
      decision: 'continue',
      reason: 'stop it',
    });
    expect(
      JSON.parse((await claudePlugin('PostToolUse', block))({ tool_name: 'Bash' }).stdout),
    ).toEqual({
      additional_context: 'stop it',
    });
    expect((await claudePlugin('SessionStart', block))({}).stdout).toBe('');
  });

  test('item 3: plain stdout is context on SessionStart and UserPromptSubmit only', async () => {
    const plain = answering('remember this');
    expect(JSON.parse((await claudePlugin('SessionStart', plain))({}).stdout)).toEqual({
      additional_context: 'remember this',
    });
    expect((await claudePlugin('PreToolUse', plain))({ tool_name: 'Bash' }).stdout).toBe('');
  });

  test('item 3: what the core cannot express is nothing, said on stderr; a denial without reason gets one', async () => {
    const ask = answering(
      json({ hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'ask' } }),
    );
    const run = (await claudePlugin('PreToolUse', ask))({ tool_name: 'Bash' });
    expect(run).toMatchObject({ status: 0, stdout: '' });
    expect(run.stderr).toContain('gave a result the core cannot express');
    const rewrite = answering(
      json({
        hookSpecificOutput: { permissionDecision: 'allow', updatedInput: { command: 'ls' } },
      }),
    );
    expect((await claudePlugin('PreToolUse', rewrite))({ tool_name: 'Bash' }).stdout).toBe('');
    const bare = answering(json({ hookSpecificOutput: { permissionDecision: 'deny' } }));
    expect(
      JSON.parse((await claudePlugin('PreToolUse', bare))({ tool_name: 'Bash' }).stdout),
    ).toEqual({
      decision: 'deny',
      reason: 'Blocked by a hook',
    });
  });

  test('item 4: another failing status is nothing, exit 0', async () => {
    const run = (await claudePlugin('Stop', answering('', '1', 'oops')))({});
    expect(run).toMatchObject({ status: 0, stdout: '' });
    expect(run.stderr).toContain('failed with status 1: oops');
  });
});
