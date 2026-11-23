/**
 * code-mode.md (2.1.330) § "Tool call refused by a hook" and § "Code Mode limitations" (Hooks):
 * when a `PreToolUse` hook gives no decision on a call from Code Mode's code, the client refuses the
 * call with `the hook did not finish` (a timeout, or an agent hook that ended without a decision)
 * when `onFailure` is unset; from 2.1.329 another failure lets the call run. Agent Plugins Extended
 * §7.6.11.4, §7.4.2 item 2, Judgment J7.213.
 */
import { expect, test } from 'bun:test';
import type { RawRun } from '../../../src/hooks/handlers/index.ts';
import { verdictOf } from '../../support/hooks-verdict.ts';
import { spec } from '../../support/spec.ts';

const TIMED_OUT: RawRun = { kind: 'timed-out' };
const guard = (onFailure?: 'block' | 'continue') =>
  ({
    type: 'command',
    command: './guard.sh',
    ...(onFailure === undefined ? {} : { onFailure }),
  }) as const;

spec(
  ['hooks.code-mode.Tool call refused by a hook', 'hooks.code-mode.Code Mode limitations.Hooks'],
  () => {
    test('a timeout with onFailure unset refuses a call from code: "the hook did not finish"', () => {
      const verdict = verdictOf('PreToolUse', TIMED_OUT, { hook: guard(), fromCode: true });
      expect(verdict.status).toBe('blocked');
      expect(verdict.block?.reason).toContain('the hook did not finish');
      expect(verdict.block?.to).toBe('claude');
    });

    test('the same timeout on a call Claude made directly lets the call go on', () => {
      expect(verdictOf('PreToolUse', TIMED_OUT, { hook: guard() }).block).toBeUndefined();
    });

    test('"continue" lets the call run after a timeout; "block" stops it', () => {
      expect(
        verdictOf('PreToolUse', TIMED_OUT, { hook: guard('continue'), fromCode: true }).block,
      ).toBeUndefined();
      expect(
        verdictOf('PreToolUse', TIMED_OUT, { hook: guard('block'), fromCode: true }).block,
      ).toBeDefined();
    });

    test('an agent hook that ends without a decision refuses the call from code', () => {
      const run: RawRun = { kind: 'model-failed', message: 'no verdict' };
      const agent = { type: 'agent', prompt: 'Check $ARGUMENTS' } as const;
      const verdict = verdictOf('PreToolUse', run, { hook: agent, fromCode: true });
      expect(verdict.block?.reason).toContain('the hook did not finish');
    });

    test('from 2.1.329 a hook that failed otherwise lets a call from code run by default', () => {
      const failed: RawRun = { kind: 'exited', exitCode: 1, stdout: '', stderr: 'crash' };
      expect(
        verdictOf('PreToolUse', failed, { hook: guard(), fromCode: true }).block,
      ).toBeUndefined();
      const notStarted: RawRun = { kind: 'not-started', message: 'ENOENT' };
      expect(
        verdictOf('PreToolUse', notStarted, { hook: guard(), fromCode: true }).block,
      ).toBeUndefined();
    });
  },
);
