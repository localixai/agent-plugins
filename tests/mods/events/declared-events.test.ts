/**
 * The events Claude Code 2.1.330 adds or changes outside the tool events (spec §10.19.2, §10.19.3,
 * §10.19.4, §10.16 "The declarations add two tool events"; reference.md § Session, § Turns,
 * § Subagents). `session.stop` (`SessionStopInput`, claude-code/index.d.ts:5900) is documented;
 * `tool.offer` (:5400), `agent.report` (:5219) and `ui.pointer` (:5134) are the declarations' only
 * (spec Judgment J10.22), so their tests run outside `spec(...)` (AGENTS.md, E2a).
 */
import { describe, expect, test } from 'bun:test';
import { EVENT_RULES, eventInputProblem } from '../../../src/mods/events/index.ts';
import { fn, harness } from '../../support/mods-chain.ts';
import { spec } from '../../support/spec.ts';

const typed = () => harness([EVENT_RULES]);
const PROVIDER = { plugin: 'engine', tier: 'core' };
const offered = () => Promise.resolve({ isOffered: true });

spec('mods.event.session.stop', () => {
  test('a hook observes the stop: it sees isTurnRunning and next(e) resolves { isTurnRunning }', async () => {
    const h = typed();
    const seen: unknown[] = [];
    h.mod('watch', 'user', [fn('session.stop', (_$, e, next) => (seen.push(e), next(e)))]);
    const result = await h.fire(
      'session.stop',
      { isTurnRunning: true },
      { core: (e) => Promise.resolve(e) },
    );
    expect(seen).toEqual([{ isTurnRunning: true }]);
    expect(result).toEqual({ isTurnRunning: true });
  });

  test('the event carries isTurnRunning as a boolean', () => {
    expect(eventInputProblem('session.stop', { isTurnRunning: false })).toBeUndefined();
    expect(eventInputProblem('session.stop', {})).toContain('session.stop input: isTurnRunning');
  });
});

describe('fields Claude Code 2.1.330 adds to documented events', () => {
  spec('mods.event.agent.offer', () => {
    test('agent.offer carries agentId on the offer to resume an agent of that type', () => {
      const offer = { agent: 'Explore', description: 'd', source: 'built-in', provider: PROVIDER };
      expect(eventInputProblem('agent.offer', { ...offer, agentId: 'a-1' })).toBeUndefined();
      expect(eventInputProblem('agent.offer', { ...offer, agentId: 7 })).toContain('agentId');
    });
  });

  spec('mods.event.session.compact', () => {
    test('each message of session.compact may carry its uuid', () => {
      const message = { role: 'user', text: 'hi', toolUses: [], uuid: 'u-1' };
      const compact = { trigger: 'auto', messages: [message] };
      expect(eventInputProblem('session.compact', compact)).toBeUndefined();
      expect(
        eventInputProblem('session.compact', { ...compact, messages: [{ ...message, uuid: 1 }] }),
      ).toContain('uuid');
    });
  });

  spec('mods.event.turn.complete', () => {
    test('usage holds ephemeral_5m_input_tokens and ephemeral_1h_input_tokens together or not at all', () => {
      const done = { turnId: 't', answer: 'a', durationMs: 1, isAborted: false, reason: 'answer' };
      const both = { ephemeral_5m_input_tokens: 3, ephemeral_1h_input_tokens: 4 };
      expect(eventInputProblem('turn.complete', { ...done, usage: both })).toBeUndefined();
      expect(eventInputProblem('turn.complete', { ...done, usage: {} })).toBeUndefined();
      expect(
        eventInputProblem('turn.complete', { ...done, usage: { ephemeral_5m_input_tokens: 3 } }),
      ).toContain('ephemeral_5m_input_tokens and ephemeral_1h_input_tokens come together');
    });
  });

  spec('mods.event.session.measure', () => {
    test('cost is { usd, hasUnknownPrice? }', () => {
      const measure = { changed: ['cost'], context: {}, rateLimits: [] };
      expect(
        eventInputProblem('session.measure', {
          ...measure,
          cost: { usd: 1.5, hasUnknownPrice: true },
        }),
      ).toBeUndefined();
      expect(
        eventInputProblem('session.measure', { ...measure, cost: { usd: 1, hasUnknownPrice: 1 } }),
      ).toContain('hasUnknownPrice');
    });
  });
});

describe('tool.offer (declarations, J10.22)', () => {
  const offer = { tool: 'Bash', provider: PROVIDER };

  test('{ isOffered: false } keeps the tool from the loop; next(e) resolves { isOffered: true }', async () => {
    const h = typed();
    h.mod('trim', 'user', [
      fn('tool.offer', (_$, e, next) => (e.tool === 'Bash' ? { isOffered: false } : next(e))),
    ]);
    expect(await h.fire('tool.offer', offer, { core: offered })).toEqual({ isOffered: false });
    expect(await h.fire('tool.offer', { ...offer, tool: 'Read' }, { core: offered })).toEqual({
      isOffered: true,
    });
  });

  test('a hook that fails passes the tool through', async () => {
    const h = typed();
    h.mod('trim', 'user', [fn('tool.offer', () => ({ isOffered: 'no' }))]);
    expect(
      await h.fire('tool.offer', offer, { core: () => Promise.resolve({ isOffered: true }) }),
    ).toEqual({ isOffered: true });
  });
});

describe('agent.report (declarations, J10.22)', () => {
  const report = {
    agentId: 'a-1',
    subagentType: 'Explore',
    status: 'completed',
    text: 'found it',
    provider: PROVIDER,
  };

  test('what the outermost hook returns is delivered: { text } or { consumed }', async () => {
    const h = typed();
    h.mod('edit', 'user', [
      fn('agent.report', (_$, e) =>
        e.status === 'failed' ? { consumed: 'retrying' } : { text: `${String(e.text)}!` },
      ),
    ]);
    expect(await h.fire('agent.report', report)).toEqual({ text: 'found it!' });
    expect(await h.fire('agent.report', { ...report, status: 'failed' })).toEqual({
      consumed: 'retrying',
    });
  });

  test('a status other than completed, failed or stopped does not fit the event', () => {
    expect(eventInputProblem('agent.report', report)).toBeUndefined();
    expect(eventInputProblem('agent.report', { ...report, status: 'done' })).toContain('status');
  });
});

describe('ui.pointer (declarations, J10.22)', () => {
  test('a click on the backdrop: type click, x, y, scene and component Backdrop', () => {
    const click = {
      type: 'click',
      x: 10,
      y: 20,
      scene: 'aurora',
      component: 'Backdrop',
      surface: 'desktop',
      requestId: 'r',
    };
    expect(eventInputProblem('ui.pointer', click)).toBeUndefined();
    expect(eventInputProblem('ui.pointer', { ...click, type: 'drag' })).toContain('type');
  });
});
