/**
 * How the reasons of several core handlers join (Agent Plugins Extended §18.1.2, "Failures, limits
 * and presentation of core handlers" item 6; Judgment J18.22): in handler order, an absent or empty
 * reason left out, one newline between them, after Claude Code's message where both appear. Chapter
 * 18 carries no requirement ids, so these are judgment-style tests outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import { idleOutcome } from '../../src/hooks/assemble.ts';
import { combineCore, type CoreSide } from '../../src/hooks/core/combine.ts';
import type { CoreResult, CoreRun } from '../../src/hooks/core/types.ts';

const runOf = (index: number, result: CoreResult): CoreRun => ({
  handlerId: `core:p:Stop:${index}`,
  pluginId: 'p',
  label: './hooks/check',
  result,
  timeoutMs: 600_000,
});

const sideOf = (event: 'Stop' | 'PreToolUse', results: readonly CoreResult[]): CoreSide => ({
  occurrence: { event, input: {} },
  runs: results.map((result, index) => runOf(index, result)),
  contexts: [],
  issues: [],
});

describe('§18.1.2 item 6, J18.22: reasons from several core handlers', () => {
  test('empty and absent reasons are left out; the others join by one newline', () => {
    const side = sideOf('Stop', [
      { decision: 'continue', reason: 'first' },
      { decision: 'continue', reason: '' },
      { decision: 'continue' },
      { decision: 'continue', reason: 'second' },
    ]);
    const outcome = combineCore(idleOutcome('Stop', []), side, {});
    expect(outcome.block?.reason).toBe('first\nsecond');
  });

  test('a decision no core handler gave a reason for carries an empty reason', () => {
    const side = sideOf('PreToolUse', [{ decision: 'deny' }, { decision: 'deny', reason: '' }]);
    const outcome = combineCore(idleOutcome('PreToolUse', []), side, {});
    expect(outcome.proceed).toBe(false);
    expect(outcome.block?.reason).toBe('');
  });
});
