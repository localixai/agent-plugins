/**
 * The hook adapter that runs a core handler under Claude Code (Agent Plugins Extended §18.4.4,
 * J18.17), started with `node` as Claude Code starts a command hook: the handler started as the core
 * expects (item 1), its core input built by §18.2 with nothing invented (item 2), its result and exit
 * status translated by meaning, a failure never blocking (item 3, J18.4). Outside `spec(...)`:
 * chapter 18 has no requirement ids.
 */
import { describe, expect, test } from 'bun:test';
import { existsSync, realpathSync } from 'node:fs';
import { join } from 'node:path';
import {
  ANSWER,
  ECHO,
  hookArgs,
  NODE,
  PROFILE,
  runNode,
  writeTree,
} from '../support/adapter-run.ts';
import { convertTree, outputOf, written } from '../support/release-guard.ts';

const schema = (document: string) =>
  `https://agent-plugins.org/schemas/1.1.0/${document}.schema.json`;
const json = (value: unknown) => JSON.stringify(value);

const handler = (script: string, ...args: string[]) => ({
  command: 'node',
  args: [`\${PLUGIN_ROOT}/handlers/${script}`, ...args],
});

async function corePackage(hooks: Record<string, unknown>) {
  const tree = {
    'plugin.json': json({ $schema: schema('plugin'), name: 'probe' }),
    'hooks.json': json({ $schema: schema('hooks'), hooks }),
    'handlers/echo.mjs': ECHO,
    'handlers/profile.mjs': PROFILE,
    'handlers/answer.mjs': ANSWER,
  };
  const converted = await convertTree(tree);
  const root = realpathSync(writeTree(written(tree, converted)));
  const hooksFile = outputOf(converted, 'hooks/hooks.json');
  const env = { CLAUDE_PLUGIN_ROOT: root, CLAUDE_PLUGIN_DATA: join(root, 'data') };
  return {
    root,
    run: (event: string, input: Record<string, unknown>) =>
      runNode(
        hookArgs(hooksFile, event).map((arg) => arg.replaceAll('${CLAUDE_PLUGIN_ROOT}', root)),
        { session_id: 's1', cwd: '/work', ...input },
        { cwd: '/', env },
      ),
  };
}

const contextOf = (stdout: string): unknown =>
  JSON.parse(JSON.parse(stdout).hookSpecificOutput.additionalContext);

describe.skipIf(NODE === null)('a core handler under Claude Code', () => {
  test('item 1: the plugin root as working directory, PLUGIN_ROOT and PLUGIN_DATA set, the data directory made', async () => {
    const pkg = await corePackage({ SessionStart: [handler('echo.mjs')] });
    const run = pkg.run('SessionStart', { hook_event_name: 'SessionStart', source: 'startup' });
    expect(contextOf(run.stdout)).toEqual({
      input: { hook_event_name: 'SessionStart', session_id: 's1', cwd: '/work' },
      cwd: pkg.root,
      root: pkg.root,
      data: join(pkg.root, 'data'),
    });
    expect(existsSync(join(pkg.root, 'data'))).toBe(true);
    expect(JSON.parse(run.stdout).hookSpecificOutput.hookEventName).toBe('SessionStart');
  });

  test('J18.6: SessionStart does not fire on resume or in a subagent', async () => {
    const pkg = await corePackage({ SessionStart: [handler('echo.mjs')] });
    const resume = pkg.run('SessionStart', { hook_event_name: 'SessionStart', source: 'resume' });
    const sub = pkg.run('SessionStart', {
      hook_event_name: 'SessionStart',
      source: 'startup',
      agent_id: 'a',
    });
    expect([resume.stdout, sub.stdout]).toEqual(['', '']);
  });

  test('J18.23: UserPromptSubmit fires on system and on no source; only a wakeup is skipped', async () => {
    const answer = json({ decision: 'deny', reason: 'not now' });
    const pkg = await corePackage({ UserPromptSubmit: [handler('answer.mjs', answer)] });
    const prompt = { hook_event_name: 'UserPromptSubmit', prompt: 'hi' };
    const block = { decision: 'block', reason: 'not now' };
    expect(JSON.parse(pkg.run('UserPromptSubmit', prompt).stdout)).toEqual(block);
    expect(JSON.parse(pkg.run('UserPromptSubmit', { ...prompt, source: 'system' }).stdout)).toEqual(
      block,
    );
    expect(pkg.run('UserPromptSubmit', { ...prompt, source: 'loop_wakeup' }).stdout).toBe('');
    expect(pkg.run('UserPromptSubmit', { ...prompt, source: 'schedule_wakeup' }).stdout).toBe('');
  });

  test('PreToolUse: the profiles of §18.2.7 from the Claude Code call', async () => {
    const pkg = await corePackage({ PreToolUse: [handler('profile.mjs')] });
    const profile = (tool_name: string, tool_input: unknown) =>
      JSON.parse(
        JSON.parse(
          pkg.run('PreToolUse', {
            hook_event_name: 'PreToolUse',
            tool_use_id: 't',
            tool_name,
            tool_input,
          }).stdout,
        ).hookSpecificOutput.permissionDecisionReason,
      );
    expect(profile('Read', { file_path: '/a.txt' })).toEqual({
      name: 'file.read',
      input: { path: '/a.txt' },
    });
    expect(profile('NotebookEdit', { notebook_path: 'n.ipynb', new_source: '' })).toEqual({
      name: 'file.modify',
      input: { operations: [{ kind: 'edit', path: '/work/n.ipynb' }] },
    });
    expect(profile('WebFetch', { url: 'docs/x' })).toBeNull();
    expect(profile('mcp__plugin_probe_notes__latest_tag', { a: 1 })).toEqual({
      name: 'mcp',
      input: { name: 'latest_tag', arguments: { a: 1 } },
    });
    // J18.21: `__` twice after `mcp__` leaves the declared name open, so no profile.
    expect(profile('mcp__plugin_probe_notes__latest__tag', { a: 1 })).toBeNull();
    expect(profile('Grep', { pattern: 'x' })).toBeNull();
  });

  test('PreToolUse: allow is an allow without updatedInput', async () => {
    const pkg = await corePackage({
      PreToolUse: [handler('answer.mjs', json({ decision: 'allow' }))],
    });
    const run = pkg.run('PreToolUse', {
      hook_event_name: 'PreToolUse',
      tool_name: 'Bash',
      tool_input: { command: 'ls' },
    });
    expect(JSON.parse(run.stdout)).toEqual({
      hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'allow' },
    });
  });

  test('J18.8: PostToolUseFailure is the core PostToolUse, with the failure as its result; an interrupt is not', async () => {
    const pkg = await corePackage({ PostToolUse: [handler('echo.mjs')] });
    const failure = {
      hook_event_name: 'PostToolUseFailure',
      tool_use_id: 't',
      tool_name: 'Bash',
      tool_input: { command: 'make' },
      error: 'Exit code 3\nboom',
    };
    const run = pkg.run('PostToolUseFailure', failure);
    expect(JSON.parse(run.stdout).hookSpecificOutput.hookEventName).toBe('PostToolUseFailure');
    expect(contextOf(run.stdout)).toMatchObject({
      input: {
        hook_event_name: 'PostToolUse',
        tool_result: 'Exit code 3\nboom',
        tool_profile: {
          name: 'shell',
          input: { command: 'make' },
          result: { output: 'boom', exit_code: 3 },
        },
      },
    });
    expect(pkg.run('PostToolUseFailure', { ...failure, is_interrupt: true }).stdout).toBe('');
  });

  test('J18.16: a text Read gets no profile (the text Claude received is not rebuilt); no Claude Code field is given', async () => {
    const pkg = await corePackage({ PostToolUse: [handler('echo.mjs')] });
    const response = { type: 'text', file: { filePath: '/a', content: 'one\ntwo', startLine: 4 } };
    const run = pkg.run('PostToolUse', {
      hook_event_name: 'PostToolUse',
      transcript_path: '/t.jsonl',
      permission_mode: 'default',
      tool_use_id: 't',
      tool_name: 'Read',
      tool_input: { file_path: '/a' },
      tool_response: response,
    });
    const seen = contextOf(run.stdout);
    expect(Object.keys(Object(Reflect.get(Object(seen), 'input'))).toSorted()).toEqual([
      'cwd',
      'hook_event_name',
      'session_id',
      'tool_input',
      'tool_name',
      'tool_result',
      'tool_use_id',
    ]);
  });

  test('J18.21: a completed command is exit 0 only without returnCodeInterpretation', async () => {
    const pkg = await corePackage({ PostToolUse: [handler('echo.mjs')] });
    const profileOf = (tool_response: Record<string, unknown>) =>
      Reflect.get(
        Object(
          Reflect.get(
            Object(
              contextOf(
                pkg.run('PostToolUse', {
                  hook_event_name: 'PostToolUse',
                  tool_use_id: 't',
                  tool_name: 'Bash',
                  tool_input: { command: 'grep x f' },
                  tool_response,
                }).stdout,
              ),
            ),
            'input',
          ),
        ),
        'tool_profile',
      );
    expect(profileOf({ stdout: 'x', stderr: '' })).toEqual({
      name: 'shell',
      input: { command: 'grep x f' },
      result: { output: 'x', exit_code: 0 },
    });
    expect(
      profileOf({ stdout: '', stderr: '', returnCodeInterpretation: 'No matches found' }),
    ).toBeUndefined();
  });

  test('Stop: a continuation is a block, continuation_active is stop_hook_active', async () => {
    const pkg = await corePackage({
      Stop: [handler('answer.mjs', json({ decision: 'continue', reason: 'again' }))],
    });
    expect(
      JSON.parse(pkg.run('Stop', { hook_event_name: 'Stop', stop_hook_active: true }).stdout),
    ).toEqual({
      decision: 'block',
      reason: 'again',
    });
  });

  test('item 3: a failure (exit 2, {}, not JSON) is a non-blocking error, exit 1 and never 2', async () => {
    const cases = [
      handler('answer.mjs', '', '2', 'bad'),
      handler('answer.mjs', '{}'),
      handler('answer.mjs', 'plain words'),
    ];
    for (const one of cases) {
      const pkg = await corePackage({ PreToolUse: [one] });
      const run = pkg.run('PreToolUse', {
        hook_event_name: 'PreToolUse',
        tool_name: 'Bash',
        tool_input: { command: 'ls' },
      });
      expect(run.status).toBe(1);
      expect(run.stdout).toBe('');
      expect(run.stderr).toContain('core hook handler [node ');
    }
  });
});
