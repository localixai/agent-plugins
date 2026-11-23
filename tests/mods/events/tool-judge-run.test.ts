/**
 * `tool.judge` and `tool.run` (spec §10.16.5 and §10.16.6; plugins/mods/reference.md § Tools of
 * Claude Code 2.1.330; `ToolJudgeInput`, claude-code/index.d.ts:5021, and `ToolCallInput` as
 * `tool.run`'s input, :5049). `next(e)` reaches Claude Code's own behaviour once per event: the
 * classifier's verdict again, or, for a run, a `{ deny }` that starts nothing.
 */
import { describe, expect, test } from 'bun:test';
import { EVENT_RULES } from '../../../src/mods/events/index.ts';
import { runModToolCall } from '../../../src/mods/load/index.ts';
import { fn, harness } from '../../support/mods-chain.ts';
import { openToolFlow } from '../../support/mods-tool-flow.ts';
import { spec } from '../../support/spec.ts';

const typed = () => harness([EVENT_RULES]);
const JUDGED = { tool: 'Bash', input: { command: 'ls' }, tool_use_id: 'toolu_1' };
const RUN = { tool: 'Bash', command: 'ls', tool_use_id: 'toolu_1' };

spec('mods.event.tool.judge', () => {
  test('next(e) asks the classifier once however often the hook calls it, and resolves to its verdict', async () => {
    const h = typed();
    let asked = 0;
    h.mod('guard', 'user', [
      fn('tool.judge', async (_$, e, next) => {
        await next(e);
        return next(e);
      }),
    ]);
    const verdict = await h.fire('tool.judge', JUDGED, {
      core: () => ((asked += 1), Promise.resolve({ decision: 'allow', reason: 'safe' })),
    });
    expect(verdict).toEqual({ decision: 'allow', reason: 'safe' });
    expect(asked).toBe(1);
  });

  test('a hook decides in the classifier place with { decision }, and the classifier is not asked', async () => {
    const h = typed();
    let asked = 0;
    h.mod('guard', 'user', [fn('tool.judge', () => ({ decision: 'deny', reason: 'no' }))]);
    const verdict = await h.fire('tool.judge', JUDGED, {
      core: () => ((asked += 1), Promise.resolve({ decision: 'allow' })),
    });
    expect(verdict).toEqual({ decision: 'deny', reason: 'no' });
    expect(asked).toBe(0);
  });

  test('a decision other than allow, ask or deny is the wrong shape: the hook is skipped and the classifier decides', async () => {
    const h = typed();
    h.mod('guard', 'user', [fn('tool.judge', () => ({ decision: 'maybe' }))]);
    const verdict = await h.fire('tool.judge', JUDGED, {
      core: () => Promise.resolve({ decision: 'ask' }),
    });
    expect(verdict).toEqual({ decision: 'ask' });
  });

  test('a hook that fails is skipped and the classifier decides; a .catch handler can deny instead', async () => {
    const h = typed();
    h.mod('guard', 'user', [
      fn(
        'tool.judge',
        () => {
          throw new Error('boom');
        },
        { handler: () => ({ decision: 'deny' }) },
      ),
    ]);
    h.mod('loose', 'user', [
      fn('tool.judge', () => {
        throw new Error('boom');
      }),
    ]);
    const verdict = await h.fire('tool.judge', JUDGED, {
      core: () => Promise.resolve({ decision: 'allow' }),
    });
    expect(verdict).toEqual({ decision: 'deny' });
  });
});

spec('mods.event.tool.run', () => {
  test('next(e) runs the tool and resolves to its result; a second next(e) starts nothing and resolves to a { deny }', async () => {
    const h = typed();
    let runs = 0;
    const seen: unknown[] = [];
    h.mod('runner', 'user', [
      fn('tool.run', async (_$, e, next) => {
        seen.push(await next(e));
        return next(e);
      }),
    ]);
    const result = await h.fire('tool.run', RUN, {
      core: () => ((runs += 1), Promise.resolve({ result: 'files' })),
    });
    expect(runs).toBe(1);
    expect(seen).toEqual([{ result: 'files' }]);
    expect(result).toEqual({ deny: 'tool.run: the call already ran; next(e) runs it once' });
  });

  test('every field is pinned: a hook that passes a changed event to next fails, and the tool runs as allowed', async () => {
    const h = typed();
    const ran: unknown[] = [];
    let error = '';
    h.mod('runner', 'user', [
      fn('tool.run', (_$, e, next) => next({ ...e, command: 'rm -rf /' }), {
        handler: (_$, e, next) => {
          error = next.error.message ?? '';
          return next(e);
        },
      }),
    ]);
    await h.fire('tool.run', RUN, { core: (e) => (ran.push(e), Promise.resolve({ result: '' })) });
    expect(ran).toEqual([RUN]);
    expect(error).toContain('tool.run: every field of the call is pinned; "command" was changed');
  });

  test('{ result } answers in the tool place, { deny } refuses: neither runs the tool', async () => {
    const h = typed();
    let runs = 0;
    const core = () => ((runs += 1), Promise.resolve({ result: 'ran' }));
    h.mod('answer', 'user', [fn('tool.run', () => ({ result: 'cached' }), { matcher: { n: 1 } })]);
    h.mod('refuse', 'user', [fn('tool.run', () => ({ deny: 'not now' }), { matcher: { n: 2 } })]);
    expect(await h.fire('tool.run', { ...RUN, n: 1 }, { core })).toEqual({ result: 'cached' });
    expect(await h.fire('tool.run', { ...RUN, n: 2 }, { core })).toEqual({ deny: 'not now' });
    expect(runs).toBe(0);
  });

  test('a hook that fails is skipped and the tool runs', async () => {
    const h = typed();
    h.mod('runner', 'user', [
      fn('tool.run', () => {
        throw new Error('boom');
      }),
    ]);
    expect(
      await h.fire('tool.run', RUN, { core: () => Promise.resolve({ result: 'ran' }) }),
    ).toEqual({ result: 'ran' });
  });
});

describe('tool.run in the order of a tool call (spec §10.16.6)', () => {
  spec('mods.event.tool.run', () => {
    test('fires after tool.check allowed the call, with the call as allowed; its { deny } is the call error, read as the tool own', async () => {
      const trace: string[] = [];
      const { flow } = await openToolFlow(trace, [
        fn('tool.check', (_$, e, next) => (trace.push('tool.check'), next(e))),
        fn('tool.run', (_$, e) => (trace.push(`tool.run ${String(e.command)}`), { deny: 'busy' })),
      ]);
      const outcome = await runModToolCall(flow);
      expect(trace).toEqual([
        'managed',
        'PreToolUse',
        'tool.check',
        'rules',
        'classifier or PermissionRequest or prompt',
        'tool.run ls',
      ]);
      expect(outcome).toEqual({ result: { isError: true, text: 'busy' } });
    });

    test('next(e) in tool.run runs the tool; a refused call never raises tool.run', async () => {
      const trace: string[] = [];
      const { flow } = await openToolFlow(trace, [
        fn('tool.run', (_$, e, next) => (trace.push('tool.run'), next(e))),
      ]);
      expect(await runModToolCall(flow)).toEqual({ result: { stdout: 'ok' } });
      expect(trace.slice(-2)).toEqual(['tool.run', 'execute']);
      trace.length = 0;
      await runModToolCall({
        ...flow,
        afterCheck: () => Promise.resolve({ decision: 'deny', reason: 'no' }),
      });
      expect(trace).not.toContain('tool.run');
      expect(trace).not.toContain('execute');
    });
  });
});
