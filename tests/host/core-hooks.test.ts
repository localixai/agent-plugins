/**
 * Core packages' hook handlers in a session beside a Claude Code plugin's hooks (Agent Plugins
 * Extended §18.1.2: the table, "Inputs", "When each side runs", "Combining results", "Failures,
 * limits and presentation"; §18.1.3 item 1; §18.2; Judgments J18.1–J18.9, J18.12; core §7.3.3–§7.3.8,
 * §9.1). Chapter 18 carries no requirement ids (Claude Code has no core packages), so these are
 * judgment-style tests outside `spec(...)` (docs/SPEC.md).
 */
import { describe, expect, test } from 'bun:test';
import { createPolicyView } from '../../src/settings/index.ts';
import {
  BASH_CALL as bash,
  CORE_ID,
  coreFiles,
  coreHostOf,
  corePackage,
  coreRuns,
  coreSessionOf as sessionOf,
  stdinOf,
} from '../support/core-host.ts';
import { SESSION, signal } from '../support/host.ts';
import { layerOf } from '../support/settings-layers.ts';

describe('§18.1.2 the table: which Claude Code occurrences are core occurrences', () => {
  test('J18.6: SessionStart fires on startup, clear and fork, not on resume or compact', async () => {
    const { session, runner } = await sessionOf();
    for (const source of ['startup', 'resume', 'clear', 'compact', 'fork'] as const) {
      await session.dispatch('SessionStart', { source }, { signal: signal() });
    }
    expect(coreRuns(runner.calls)).toHaveLength(3);
  });

  test('core §7.3.4: SessionStart does not run for a subagent', async () => {
    const { host, runner } = await coreHostOf();
    const session = host.openSession({ ...SESSION, agent: { id: 'a1', type: 'Explore' } });
    await session.dispatch('SessionStart', { source: 'startup' }, { signal: signal() });
    expect(coreRuns(runner.calls)).toHaveLength(0);
  });

  test('J18.7: UserPromptSubmit fires by origin: the source decides all but system', async () => {
    const { session, runner } = await sessionOf();
    const prompt = (source?: 'user' | 'sdk' | 'poll_event' | 'loop_wakeup' | 'schedule_wakeup') =>
      session.dispatch(
        'UserPromptSubmit',
        { prompt: 'hi', ...(source === undefined ? {} : { source }) },
        { signal: signal() },
      );
    for (const source of [
      'user',
      'sdk',
      'poll_event',
      undefined,
      'loop_wakeup',
      'schedule_wakeup',
    ] as const)
      await prompt(source);
    expect(coreRuns(runner.calls)).toHaveLength(4);
    const system = (turnOrigin: 'external' | 'client') =>
      session.dispatch(
        'UserPromptSubmit',
        { prompt: 'from elsewhere', source: 'system' },
        { signal: signal(), core: { turnOrigin } },
      );
    await system('external');
    expect(coreRuns(runner.calls)).toHaveLength(5);
    await system('client');
    expect(coreRuns(runner.calls)).toHaveLength(5);
  });

  test('J18.7: a system turn without its origin from the host is refused, never defaulted', async () => {
    const { session, runner } = await sessionOf();
    const turn = { prompt: 'a task notification or a peer?', source: 'system' } as const;
    const refused = await session.dispatch('UserPromptSubmit', turn, { signal: signal() }).then(
      () => undefined,
      (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
    expect(refused).toContain('turnOrigin');
    expect(coreRuns(runner.calls)).toHaveLength(0);
    const stopOnly = coreFiles({ Stop: [{ command: './hooks/check', args: ['stop'] }] });
    const { session: without } = await sessionOf({}, { core: await corePackage(stopOnly) });
    const outcome = await without.dispatch('UserPromptSubmit', turn, { signal: signal() });
    expect(outcome.proceed).toBe(true);
  });

  test('J18.8: PostToolUseFailure is a core PostToolUse unless it is an interrupt', async () => {
    const { session, runner } = await sessionOf();
    const failure = { ...bash, error: 'Exit code 1\nboom' };
    await session.dispatch('PostToolUseFailure', failure, { signal: signal() });
    await session.dispatch(
      'PostToolUseFailure',
      { ...failure, is_interrupt: true },
      { signal: signal() },
    );
    const runs = coreRuns(runner.calls);
    expect(runs).toHaveLength(1);
    expect(stdinOf(runs[0])).toMatchObject({
      hook_event_name: 'PostToolUse',
      tool_result: 'Exit code 1\nboom',
    });
  });

  test('the 28 other Claude Code events never run a core handler', async () => {
    const { session, runner } = await sessionOf();
    await session.dispatch(
      'SubagentStop',
      { stop_hook_active: false, agent_id: 'a', agent_type: 'x', agent_transcript_path: '/t' },
      { signal: signal() },
    );
    expect(coreRuns(runner.calls)).toHaveLength(0);
  });
});

describe('§18.1.2 "Inputs" and core §7.3.3: exactly the core fields', () => {
  test('a SessionStart handler gets the three common fields and nothing of Claude Code', async () => {
    const { session, runner } = await sessionOf();
    await session.dispatch('SessionStart', { source: 'startup' }, { signal: signal() });
    expect(stdinOf(coreRuns(runner.calls)[0])).toEqual({
      hook_event_name: 'SessionStart',
      session_id: 'sess-1',
      cwd: '/work/app',
    });
  });

  test('a PreToolUse handler gets the call and its shell profile; the process is core §7.3.3', async () => {
    const { session, runner } = await sessionOf();
    await session.dispatch('PreToolUse', bash, { signal: signal() });
    const run = coreRuns(runner.calls)[0];
    expect(stdinOf(run)).toEqual({
      hook_event_name: 'PreToolUse',
      session_id: 'sess-1',
      cwd: '/work/app',
      tool_use_id: 't1',
      tool_name: 'Bash',
      tool_input: { command: 'npm publish' },
      tool_profile: { name: 'shell', input: { command: 'npm publish' } },
    });
    expect(run?.spec).toEqual({
      form: 'exec',
      command: `/plugins/${CORE_ID}/hooks/check`,
      args: ['pre', `/plugins/${CORE_ID}/policy.json`],
    });
    expect(run?.cwd).toBe(`/plugins/${CORE_ID}`);
    expect(run?.timeoutMs).toBe(600_000);
  });

  test('J18.5: the UserPromptSubmit limit is 30 seconds', async () => {
    const { session, runner } = await sessionOf();
    await session.dispatch('UserPromptSubmit', { prompt: 'hi' }, { signal: signal() });
    expect(coreRuns(runner.calls)[0]?.timeoutMs).toBe(30_000);
  });

  test('J18.9: continuation_active is stop_hook_active', async () => {
    const { session, runner } = await sessionOf();
    await session.dispatch('Stop', { stop_hook_active: true }, { signal: signal() });
    expect(stdinOf(coreRuns(runner.calls)[0])).toMatchObject({ continuation_active: true });
  });
});

describe('§18.1.3 item 1, J18.12: the environment of a core handler', () => {
  test('PLUGIN_ROOT and PLUGIN_DATA are set, CLAUDE_PLUGIN_* removed, CLAUDE_PROJECT_DIR set', async () => {
    const env = { PATH: '/bin', CLAUDE_PLUGIN_ROOT: '/inherited', CLAUDE_PLUGIN_OPTION_X: 'y' };
    const { session, runner } = await sessionOf({}, { env });
    await session.dispatch('SessionStart', { source: 'startup' }, { signal: signal() });
    const core = coreRuns(runner.calls)[0]?.env ?? {};
    expect(core).toMatchObject({
      PATH: '/bin',
      PLUGIN_ROOT: `/plugins/${CORE_ID}`,
      PLUGIN_DATA: `/data/${CORE_ID}`,
      CLAUDE_PROJECT_DIR: '/work/app',
      CLAUDE_CODE_SESSION_ID: 'sess-1',
      CLAUDECODE: '1',
    });
    expect(Object.keys(core).filter((name) => name.startsWith('CLAUDE_PLUGIN_'))).toEqual([]);
  });

  test('§18.1.3 item 2: a Claude Code hook gets no PLUGIN_ROOT or PLUGIN_DATA', async () => {
    const { session, runner } = await sessionOf();
    await session.dispatch('SessionStart', { source: 'startup' }, { signal: signal() });
    const claude = runner.calls.find((run) => run.spec.command === 'cc-start')?.env ?? {};
    expect(claude['PLUGIN_ROOT']).toBeUndefined();
    expect(claude['PLUGIN_DATA']).toBeUndefined();
  });
});

describe('§18.1.2 "Failures": a core failure never blocks (J18.4)', () => {
  test('exit 2 from a core handler is a failure with a notice, never a denial', async () => {
    const { session } = await sessionOf({ 'core-pre': { exitCode: 2, stderr: 'nope' } });
    const outcome = await session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(outcome.proceed).toBe(true);
    expect(outcome.notices.map((notice) => notice.message)).toEqual([
      // J18.22: the label is `command` and `args` as written in hooks.json, not as they ran.
      'PreToolUse:Bash hook error: [./hooks/check pre ${PLUGIN_ROOT}/policy.json]: Failed with non-blocking status code: nope',
    ]);
  });

  test('plain text, {} and a shape of another event are failures, never context', async () => {
    for (const stdout of ['some text', '{}', JSON.stringify({ decision: 'deny', reason: 'x' })]) {
      const { session } = await sessionOf({ 'core-start': { stdout } });
      const outcome = await session.dispatch(
        'SessionStart',
        { source: 'startup' },
        { signal: signal() },
      );
      expect(outcome.additionalContext).toEqual([]);
      expect(outcome.notices).toHaveLength(1);
    }
  });

  test('whitespace only is no opinion', async () => {
    const { session } = await sessionOf({ 'core-start': { stdout: ' \n\t' } });
    const outcome = await session.dispatch(
      'SessionStart',
      { source: 'startup' },
      { signal: signal() },
    );
    expect(outcome.notices).toEqual([]);
  });
});

describe('§18.1.2 item 5, J18.15: hook policy applies to core handlers', () => {
  test('disableAllHooks turns them off', async () => {
    const settings = createPolicyView([layerOf('user', { disableAllHooks: true })]);
    const { session, runner } = await sessionOf({}, { settings });
    await session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(coreRuns(runner.calls)).toHaveLength(0);
  });

  test('allowManagedHooksOnly keeps only a force-enabled package', async () => {
    const off = createPolicyView([layerOf('managed', { allowManagedHooksOnly: true })]);
    const blocked = await sessionOf({}, { settings: off });
    await blocked.session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(coreRuns(blocked.runner.calls)).toHaveLength(0);
    const forced = createPolicyView([
      layerOf('managed', { allowManagedHooksOnly: true, enabledPlugins: { [CORE_ID]: true } }),
    ]);
    const kept = await sessionOf({}, { settings: forced });
    await kept.session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(coreRuns(kept.runner.calls)).toHaveLength(1);
  });

  test('a package enabledPlugins turns off runs no handler', async () => {
    const settings = createPolicyView([layerOf('user', { enabledPlugins: { [CORE_ID]: false } })]);
    const { session, runner } = await sessionOf({}, { settings });
    await session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(coreRuns(runner.calls)).toHaveLength(0);
  });
});
