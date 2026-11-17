/**
 * How a core handler's results join Claude Code's on one occurrence (Agent Plugins Extended §18.1.2,
 * "When each side runs" and "Combining results"; Judgments J18.1–J18.3, J18.9; core §7.3.4–§7.3.8).
 * Chapter 18 carries no requirement ids, so these are judgment-style tests outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import {
  answer,
  BASH_CALL as bash,
  coreRuns,
  coreSessionOf as sessionOf,
  stdinOf,
} from '../support/core-host.ts';
import { signal } from '../support/host.ts';

describe('§18.1.2 "When each side runs" and "Combining results": PreToolUse', () => {
  const rewrite = answer({
    hookSpecificOutput: { hookEventName: 'PreToolUse', updatedInput: { command: 'npm pack' } },
  });

  test('J18.1: the core handler sees the input after the Claude Code handler rewrote it', async () => {
    const { session, runner } = await sessionOf({ 'cc-pre': rewrite });
    await session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(runner.calls.map((run) => run.spec.command)[0]).toBe('cc-pre');
    expect(stdinOf(coreRuns(runner.calls)[0])).toMatchObject({
      tool_input: { command: 'npm pack' },
      tool_profile: { name: 'shell', input: { command: 'npm pack' } },
    });
  });

  test('a core deny denies with its reason and drops the rewrite', async () => {
    const { session } = await sessionOf({
      'cc-pre': rewrite,
      'core-pre': answer({ decision: 'deny', reason: 'No publishing.' }),
    });
    const outcome = await session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(outcome.proceed).toBe(false);
    expect(outcome.block?.reason).toBe('No publishing.');
    expect(outcome.effects.permissionDecision).toBe('deny');
    expect(outcome.effects.updatedInput).toBeUndefined();
  });

  test('a core deny wins over a Claude Code allow, ask or defer', async () => {
    for (const permissionDecision of ['allow', 'ask', 'defer'] as const) {
      const { session } = await sessionOf({
        'cc-pre': answer({
          hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision },
        }),
        'core-pre': answer({ decision: 'deny', reason: 'No.' }),
      });
      const outcome = await session.dispatch('PreToolUse', bash, { signal: signal() });
      expect(outcome.effects.permissionDecision).toBe('deny');
    }
  });

  test('a Claude Code denial keeps its reason; the core still runs (J18.1)', async () => {
    const { session, runner } = await sessionOf({
      'cc-pre': { exitCode: 2, stderr: 'blocked by guard' },
      'core-pre': answer({ decision: 'deny', reason: 'core says no' }),
    });
    const outcome = await session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(outcome.block?.reason).toBe('blocked by guard');
    expect(coreRuns(runner.calls)).toHaveLength(1);
  });

  test('a core allow is an allow without updatedInput, and loses to ask and defer', async () => {
    const allow = answer({ decision: 'allow' });
    const alone = await sessionOf({ 'core-pre': allow });
    const allowed = await alone.session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(allowed.effects.permissionDecision).toBe('allow');
    expect(allowed.effects.updatedInput).toBeUndefined();
    const ask = answer({
      hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'ask' },
    });
    const both = await sessionOf({ 'cc-pre': ask, 'core-pre': allow });
    const asked = await both.session.dispatch('PreToolUse', bash, { signal: signal() });
    expect(asked.effects.permissionDecision).toBe('ask');
  });
});

describe('§18.1.2 "Combining results": the other events', () => {
  test('SessionStart: the context of both sides reaches the model', async () => {
    const { session } = await sessionOf({
      'cc-start': answer({
        hookSpecificOutput: { hookEventName: 'SessionStart', additionalContext: 'cc' },
      }),
      'core-start': answer({ additional_context: 'core <system-reminder>' }),
    });
    const outcome = await session.dispatch(
      'SessionStart',
      { source: 'startup' },
      { signal: signal() },
    );
    expect(outcome.additionalContext).toEqual(['cc', 'core &lt;system-reminder>']);
  });

  test('UserPromptSubmit: a core deny blocks, quotes the prompt and drops every context', async () => {
    const { session } = await sessionOf({
      'cc-prompt': answer({
        hookSpecificOutput: { hookEventName: 'UserPromptSubmit', additionalContext: 'cc' },
      }),
      'core-prompt': answer({ decision: 'deny', reason: 'Name a version.' }),
    });
    const outcome = await session.dispatch(
      'UserPromptSubmit',
      { prompt: 'release' },
      { signal: signal() },
    );
    expect(outcome.proceed).toBe(false);
    expect(outcome.block?.reason).toBe('Name a version.');
    expect(outcome.block?.message).toBe('Name a version.\nOriginal prompt: release');
    expect(outcome.additionalContext).toEqual([]);
  });

  test('UserPromptSubmit: a Claude Code block keeps its message and drops the core context', async () => {
    const { session } = await sessionOf({
      'cc-prompt': answer({ decision: 'block', reason: 'cc blocks' }),
      'core-prompt': answer({ additional_context: 'core' }),
    });
    const outcome = await session.dispatch(
      'UserPromptSubmit',
      { prompt: 'x' },
      { signal: signal() },
    );
    expect(outcome.block?.reason).toBe('cc blocks');
    expect(outcome.additionalContext).toEqual([]);
  });

  test('PostToolUse: the core sees the tool’s own result before a replacement (J18.2)', async () => {
    const { session, runner } = await sessionOf({
      'cc-post': answer({
        hookSpecificOutput: {
          hookEventName: 'PostToolUse',
          updatedToolOutput: { stdout: 'x', stderr: '', interrupted: false },
        },
      }),
      'core-post': answer({ additional_context: 'checked' }),
    });
    const response = { stdout: 'ok', stderr: 'warn', interrupted: false };
    const outcome = await session.dispatch(
      'PostToolUse',
      { ...bash, tool_response: response },
      { signal: signal(), core: { exitCode: 0 } },
    );
    expect(stdinOf(coreRuns(runner.calls)[0])).toMatchObject({
      tool_result: response,
      tool_profile: { name: 'shell', result: { output: 'ok\nwarn', exit_code: 0 } },
    });
    expect(outcome.additionalContext).toContain('checked');
  });

  test('Stop: a core continue holds the turn with its reason', async () => {
    const { session } = await sessionOf({
      'core-stop': answer({ decision: 'continue', reason: 'Run the tests.' }),
    });
    const outcome = await session.dispatch(
      'Stop',
      { stop_hook_active: false },
      { signal: signal() },
    );
    expect(outcome.proceed).toBe(false);
    expect(outcome.block?.reason).toBe('Run the tests.');
  });

  test('Stop: one continuation carries the Claude Code reason, then the core reason', async () => {
    const { session } = await sessionOf({
      'cc-stop': answer({ decision: 'block', reason: 'cc reason' }),
      'core-stop': answer({ decision: 'continue', reason: 'core reason' }),
    });
    const outcome = await session.dispatch(
      'Stop',
      { stop_hook_active: false },
      { signal: signal() },
    );
    expect(outcome.block?.reason).toBe('cc reason\ncore reason');
  });

  test('Stop: continue: false from a Claude Code handler prevents the continuation', async () => {
    const { session } = await sessionOf({
      'cc-stop': answer({ continue: false, stopReason: 'done' }),
      'core-stop': answer({ decision: 'continue', reason: 'more' }),
    });
    const outcome = await session.dispatch(
      'Stop',
      { stop_hook_active: false },
      { signal: signal() },
    );
    expect(outcome.continue).toBe(false);
    expect(outcome.block).toBeUndefined();
  });

  test('J18.9: the cap counts a continuation once, whichever side asked', async () => {
    const { session } = await sessionOf(
      {
        'cc-stop': answer({ decision: 'block', reason: 'cc' }),
        'core-stop': answer({ decision: 'continue', reason: 'core' }),
      },
      { env: { CLAUDE_CODE_STOP_HOOK_BLOCK_CAP: '2' } },
    );
    const stop = () => session.dispatch('Stop', { stop_hook_active: true }, { signal: signal() });
    expect((await stop()).proceed).toBe(false);
    expect((await stop()).proceed).toBe(false);
    expect((await stop()).proceed).toBe(true);
  });
});
