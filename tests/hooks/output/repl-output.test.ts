/**
 * code-mode.md (2.1.330) § "Check code output with PostToolUse": a `PostToolUse` hook that matches
 * `REPL` reads the code's output and redacts it with `updatedToolOutput` in one shape only ("Change
 * the text", "Keep the shape", "Remove images and PDFs"), with the block and error cases of "Redact
 * when your hook blocks or hits an error". Agent Plugins Extended §7.6.11.3 (Judgment J7.214: a
 * client MAY offer Code Mode; the kit checks the shape for a host that does).
 */
import { describe, expect, test } from 'bun:test';
import { FACTS } from '../../support/hooks-select.ts';
import { merged, specific, viaEngine } from '../../support/hooks-output.ts';
import { build } from '../../support/hooks-input.ts';
import { spec } from '../../support/spec.ts';

const ORIGINAL = {
  code: 'const r = await tools.Bash({ command: "cat .env" }); return r.stdout;',
  result: 'API_KEY=sk-123',
  stdout: 'API_KEY=sk-123\n',
  stderr: '',
  toolsUsed: ['Bash'],
  images: [{ type: 'image', data: 'iVBOR' }],
  documents: [{ type: 'document', data: 'JVBER' }],
  refusedCalls: [{ tool: 'Bash', input: { command: 'cat ~/.ssh/id_rsa' } }],
};

const facts = {
  ...FACTS.PostToolUse,
  tool_name: 'REPL',
  tool_input: { code: ORIGINAL.code },
  tool_response: ORIGINAL,
};

const rewrite = (updatedToolOutput: unknown, more: object = {}) => ({
  ...specific('PostToolUse', { updatedToolOutput }),
  ...more,
});

const outputOf = (copy: unknown, more: object = {}) =>
  merged('PostToolUse', [rewrite(copy, more)], facts).effects.updatedToolOutput;

const REDACTED = { ...ORIGINAL, result: '[redacted]', stdout: '[redacted]\n' };

spec('hooks.code-mode.Read code output in the hook', () => {
  test('the hook receives code, result, stdout, stderr and toolsUsed in tool_response', () => {
    const { wire, issues } = build('PostToolUse', facts);
    expect(issues).toEqual([]);
    expect(wire?.['tool_name']).toBe('REPL');
    expect(wire?.['tool_response']).toEqual(ORIGINAL);
  });

  test('a PreToolUse hook that matches REPL receives the JavaScript in tool_input.code', async () => {
    const pre = { ...FACTS.PreToolUse, tool_name: 'REPL', tool_input: { code: ORIGINAL.code } };
    expect(build('PreToolUse', pre).wire?.['tool_input']).toEqual({ code: ORIGINAL.code });
    const deny = specific('PreToolUse', { permissionDecision: 'deny' });
    expect((await viaEngine('PreToolUse', [deny], pre)).proceed).toBe(false);
  });
});

spec('hooks.code-mode.Redact code output.Change the text', () => {
  test('a copy with the text of result, stdout, stderr and error changed is accepted', () => {
    expect(outputOf(REDACTED)).toEqual(REDACTED);
    const failed = { ...ORIGINAL, error: '[redacted]', stderr: '[redacted]' };
    expect(outputOf(failed)).toEqual(failed);
  });

  test('result can be missing or null', () => {
    const { result: _dropped, ...withoutResult } = REDACTED;
    expect(outputOf(withoutResult)).toEqual(withoutResult);
    expect(outputOf({ ...REDACTED, result: null })).toEqual({ ...REDACTED, result: null });
  });
});

spec('hooks.code-mode.Redact code output.Keep the shape', () => {
  test('code, stdout and stderr must stay strings, error a string or absent; otherwise the original stands', () => {
    for (const broken of [
      { ...REDACTED, code: 1 },
      { ...REDACTED, stdout: null },
      { ...REDACTED, stderr: ['x'] },
      { ...REDACTED, error: 42 },
      'API_KEY=[redacted]',
      { result: '[redacted]' },
    ]) {
      expect(outputOf(broken), JSON.stringify(broken)).toBeUndefined();
    }
  });
});

spec('hooks.code-mode.Redact code output.Remove images and PDFs', () => {
  test('images and documents set to [] remove them', () => {
    const stripped = { ...ORIGINAL, images: [], documents: [] };
    expect(outputOf(stripped)).toEqual(stripped);
  });

  test('images that are not a list are not the shape', () => {
    expect(outputOf({ ...ORIGINAL, images: 'none' })).toBeUndefined();
  });
});

spec('hooks.code-mode.Redact when your hook blocks or hits an error', () => {
  test('a block that carries the redacted copy hides the output; one without it leaves the original', () => {
    const blocking = { decision: 'block', reason: 'secrets in output' };
    expect(outputOf(REDACTED, blocking)).toEqual(REDACTED);
    const withoutCopy = merged('PostToolUse', [blocking], facts);
    expect(withoutCopy.effects.updatedToolOutput).toBeUndefined();
  });
});

describe('refusedCalls (code-mode.md: "Claude reads it unchanged")', () => {
  test('a copy that changes refusedCalls is accepted with the original refusedCalls', () => {
    const changed = { ...REDACTED, refusedCalls: [] };
    expect(outputOf(changed)).toEqual(REDACTED);
  });
});
