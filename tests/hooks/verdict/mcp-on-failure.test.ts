/**
 * hooks.md (2.1.330) § "MCP tool hook fields" (`onFailure`, Claude Code 2.1.328), "How the tool's
 * result is read" (an answer the client cannot read) and "Block the action when a hook fails" (the
 * "MCP failure" row): Agent Plugins Extended §7.3.5 items 2–4 and §7.4.3 with Judgment J7.212.
 */
import { describe, expect, test } from 'bun:test';
import type { RawRun } from '../../../src/hooks/handlers/index.ts';
import type { HookEvent } from '../../../src/protocol/official.ts';
import { verdictOf } from '../../support/hooks-verdict.ts';
import { spec } from '../../support/spec.ts';

const GUARD = {
  type: 'mcp_tool',
  server: 'my_server',
  tool: 'security_scan',
  onFailure: 'block',
} as const;
const PLAIN = { type: 'mcp_tool', server: 'my_server', tool: 'security_scan' } as const;

const answered = (text: string, isError = false): RawRun => ({ kind: 'mcp', isError, text });

const TOO_LARGE =
  'MCP tool "security_scan" on server "my_server" gave no answer that could be read, because a message from the server was too large, so this hook gave no verdict. If this keeps happening, the tool must send a smaller answer.';
const NO_TEXT =
  'MCP tool "security_scan" on server "my_server" answered without its text, which happens when a result is too large, so this hook gave no verdict. If this keeps happening, the tool must send a smaller answer.';

const unreadable = (cause: 'message-too-large' | 'missing-text', text: string): RawRun => ({
  kind: 'mcp',
  isError: true,
  text,
  unreadable: cause,
});

spec('hooks.handler.mcp_tool.onFailure', () => {
  test('"block": a tool that answers isError: true blocks the call', () => {
    const verdict = verdictOf('PreToolUse', answered('scan failed', true), { hook: GUARD });
    expect(verdict.status).toBe('blocked');
    expect(verdict.block?.reason).toContain('failed; blocking because onFailure is "block"');
  });

  test('unset, or "continue": the same answer is a non-blocking error', () => {
    for (const hook of [PLAIN, { ...GUARD, onFailure: 'continue' } as const]) {
      const verdict = verdictOf('PreToolUse', answered('scan failed', true), { hook });
      expect(verdict.status).toBe('non-blocking-error');
      expect(verdict.block).toBeUndefined();
    }
  });
});

spec('hooks.behavior.Block the action when a hook fails.MCP failure', () => {
  test('a server that is not connected blocks with "block" (the handler reports it as isError)', () => {
    const text =
      'The MCP server "my_server" is not connected, so the hook did not call security_scan.';
    expect(
      verdictOf('UserPromptSubmit', answered(text, true), { hook: GUARD }).block,
    ).toBeDefined();
    expect(
      verdictOf('UserPromptSubmit', answered(text, true), { hook: PLAIN }).block,
    ).toBeUndefined();
  });

  test('an answer the client could not read is a failure that "block" turns into a block', () => {
    const run = unreadable('missing-text', NO_TEXT);
    expect(verdictOf('UserPromptSubmit', run, { hook: GUARD }).block).toBeDefined();
    expect(verdictOf('UserPromptSubmit', run, { hook: PLAIN }).block).toBeUndefined();
  });

  test('with "block" on an event that can block, a non-empty answer that sets no output field fails', () => {
    for (const event of ['PreToolUse', 'UserPromptSubmit'] as const) {
      for (const text of ['all clear', '{"reason":"fine"}', '{"verdict":"ok"}', '[1]']) {
        expect(verdictOf(event, answered(text), { hook: GUARD }).status, text).toBe('blocked');
      }
    }
  });

  test('{} , nothing, and JSON that sets a hook output field pass', () => {
    const allow = JSON.stringify({
      hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'allow' },
    });
    for (const text of ['', '  ', '{}', '{"systemMessage":"ok","reason":"r"}', allow]) {
      const verdict = verdictOf('PreToolUse', answered(text), { hook: GUARD });
      expect(verdict.block, text).toBeUndefined();
    }
  });

  test('without "block" a plain-text answer is no failure', () => {
    expect(verdictOf('PreToolUse', answered('all clear'), { hook: PLAIN }).status).toBe('success');
  });

  test('J7.212: on an event that cannot block, "block" makes no answer a failure', () => {
    const verdict = verdictOf('PostToolUseFailure', answered('all clear'), { hook: GUARD });
    expect(verdict.status).toBe('success');
  });
});

spec('hooks.handler.mcp_tool.tool', () => {
  test('an answer too large to read: PreToolUse refuses the call, whatever onFailure says', () => {
    for (const hook of [PLAIN, GUARD, { ...GUARD, onFailure: 'continue' } as const]) {
      const verdict = verdictOf('PreToolUse', unreadable('message-too-large', TOO_LARGE), { hook });
      expect(verdict.status).toBe('blocked');
      expect(verdict.block?.reason).toBe(TOO_LARGE);
      expect(verdict.block?.to).toBe('claude');
    }
  });

  test('a result without its text: refused on PreToolUse, a non-blocking hook error elsewhere', () => {
    const run = unreadable('missing-text', NO_TEXT);
    expect(verdictOf('PreToolUse', run, { hook: PLAIN }).block?.reason).toBe(NO_TEXT);
    const events: readonly HookEvent[] = ['UserPromptSubmit', 'PostToolUse', 'Stop'];
    for (const event of events) {
      const verdict = verdictOf(event, run, { hook: PLAIN });
      expect(verdict.block, event).toBeUndefined();
      expect(verdict.status).toBe('non-blocking-error');
      expect(verdict.notice?.message ?? verdict.feedback?.text).toContain(NO_TEXT);
    }
  });
});

describe('an unreadable answer gives no verdict', () => {
  test('nothing of the text is read as output', () => {
    const run = unreadable('missing-text', '{"decision":"block","reason":"x"}');
    expect(verdictOf('Stop', run, { hook: PLAIN }).output).toBeUndefined();
  });
});
