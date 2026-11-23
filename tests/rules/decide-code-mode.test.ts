/**
 * code-mode.md (2.1.330) § "Permission checks", "Require approval for a command", "Approve each
 * piece of code first" and "Code Mode limitations" (Auto mode); permission-modes.md § "How the
 * classifier evaluates actions" (what it reviews beyond tool calls: `SendMessage`). Agent Plugins
 * Extended §7.6.11.2 and §13.9.4 item 5; offering Code Mode is a MAY (Judgment J7.214), so a host
 * that does marks the calls its code makes with `PreparedCall.fromCode`.
 */
import { describe, expect, test } from 'bun:test';
import { decide, type PreparedCall } from '../../src/rules/index.ts';
import { toolRequiresPermission } from '../../src/rules/tools/catalog.ts';
import { baseContext, decision, rulesOf } from '../support/decide.ts';
import { spec } from '../support/spec.ts';

const AUTO = { autoAvailable: true, bypassAvailable: true };
const CODE = { code: 'const r = await tools.Bash({ command: "git push" }); return r;' };

const fromCode = (toolName: string, input: unknown): PreparedCall => ({
  toolName,
  input,
  fromCode: true,
});

const inMode = (
  mode: 'default' | 'auto' | 'bypassPermissions' | 'acceptEdits',
  call: PreparedCall,
  rules: ReturnType<typeof rulesOf> = [],
) => decide(call, baseContext(mode, { ...AUTO, rules }));

spec('hooks.code-mode.Permission checks', () => {
  test("a call from code meets deny and ask rules under the tool's usual name", () => {
    const rules = rulesOf(['deny', 'Bash(rm *)'], ['ask', 'mcp__github__create_issue']);
    for (const mode of ['default', 'auto', 'bypassPermissions'] as const) {
      expect(inMode(mode, fromCode('Bash', { command: 'rm -rf build' }), rules).outcome).toBe(
        'deny',
      );
      expect(inMode(mode, fromCode('mcp__github__create_issue', {}), rules).outcome).toBe('ask');
    }
  });

  test('in Manual mode the user is prompted for each call that would prompt there', () => {
    expect(inMode('default', fromCode('Bash', { command: 'npm publish' })).outcome).toBe('ask');
    expect(inMode('default', fromCode('Read', { file_path: '/work/app/a.ts' })).outcome).toBe(
      'allow',
    );
  });

  test('in auto mode the classifier reviews the code, not each call: a call from code runs', () => {
    expect(inMode('auto', fromCode('Bash', { command: 'npm publish' }))).toEqual({
      outcome: 'allow',
      reason: 'mode-auto',
    });
    expect(inMode('auto', { toolName: 'Bash', input: { command: 'npm publish' } }).outcome).toBe(
      'classify',
    );
  });

  test('the actions no mode auto-approves still prompt from code in auto mode', () => {
    expect(inMode('auto', fromCode('Bash', { command: 'rm -rf /' })).outcome).not.toBe('allow');
  });
});

spec('hooks.code-mode.Code Mode limitations.Auto mode', () => {
  test('the REPL call itself goes to the classifier in auto mode', () => {
    expect(decision('auto', 'REPL', CODE, AUTO).outcome).toBe('classify');
  });
});

spec('hooks.code-mode.Require approval for a command', () => {
  test('an ask rule such as Bash(git push *) prompts for a call from code, in auto and bypassPermissions too', () => {
    const rules = rulesOf(['ask', 'Bash(git push *)']);
    for (const mode of ['auto', 'bypassPermissions'] as const) {
      expect(inMode(mode, fromCode('Bash', { command: 'git push origin main' }), rules)).toEqual({
        outcome: 'ask',
        reason: 'ask-rule',
      });
    }
  });
});

spec('hooks.code-mode.Approve each piece of code first', () => {
  test('an ask rule for REPL prompts before each piece of code, in auto and bypassPermissions too', () => {
    const rules = rulesOf(['ask', 'REPL']);
    for (const mode of ['auto', 'bypassPermissions'] as const) {
      expect(decision(mode, 'REPL', CODE, { ...AUTO, rules }).outcome).toBe('ask');
    }
  });
});

describe('SendMessage in auto mode (§13.9.4 item 5)', () => {
  const message = { to: 'researcher', message: 'start on the parser' };
  const catalog = { ...AUTO, requiresPermission: toolRequiresPermission };

  test('outside Code Mode a message to another agent is reviewed before delivery', () => {
    expect(decision('auto', 'SendMessage', message, catalog).outcome).toBe('classify');
  });

  test('an allow rule that matches SendMessage, or a PreToolUse allow, skips the review', () => {
    const rules = rulesOf(['allow', 'SendMessage']);
    expect(decision('auto', 'SendMessage', message, { ...catalog, rules }).outcome).toBe('allow');
    const hook = { decision: 'allow' } as const;
    expect(decision('auto', 'SendMessage', message, { ...catalog, hook }).outcome).toBe('allow');
  });

  test('in Code Mode a message from code is not reviewed on its own', () => {
    expect(decide(fromCode('SendMessage', message), baseContext('auto', catalog)).outcome).toBe(
      'allow',
    );
  });

  test('in plan mode with auto mode in plan it is reviewed too; in Manual mode it runs', () => {
    const plan = { ...catalog, bypassAvailable: false };
    expect(decision('plan', 'SendMessage', message, plan).outcome).toBe('classify');
    expect(decision('default', 'SendMessage', message, catalog).outcome).toBe('allow');
  });
});
